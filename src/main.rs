mod assignment_utils;
mod default_files;
mod fileops_utils;
mod rand_utils;
mod settings_util;
mod sound_utils;
mod ui;

use assignment_utils::{
    convert_hashmap_to_tuplevector, create_weighttable, get_tag_counter_hashmap,
    hashmap_to_taskvector, readin_tasks, taskvector_to_stringvect, turn_assignmentvector_into_pdf,
    update_tagweights,
};
use rand_utils::roll_die;
use std::error::Error;
use std::io;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::Terminal;
use ui::event::{Event, Events};
use ui::{
    draw_current_task, draw_gauge, draw_help, draw_tag_counter, draw_task_table, draw_weights, App,
    HelpTable, WeightTable,
};

#[macro_use] extern crate pathsep;
#[macro_use] extern crate serde_derive;

// this function reads in the task list provided in
// settings and then randomly selects one task to
// perform. the function returns two string vectors.
// one corresponds to the specific task that was
// chosen, and the other corresponds to the updated
// table of tasks to display. these values are fed
// into the UI.
fn choose_task(
    // read in tasks
    //  from the task file, the vector of tags,
    //  the vector of tag weights, and the
    //  vector of booleans denoting whether or not to
    //  use tag weights
    configured_task_path: &str,
    vector_of_tags: &Vec<String>,
    initial_tag_weights: &Vec<f64>,
    configured_use_of_due_dates: &Vec<bool>,
) -> (Vec<String>, Vec<Vec<String>>, Vec<Vec<String>>) {
    //) -> (Vec<String>, Vec<Vec<String>>) {
    let tag_to_vector_map = readin_tasks(configured_task_path, &vector_of_tags);

    let configured_relative_tag_weights =
        update_tagweights(&tag_to_vector_map, initial_tag_weights, vector_of_tags);

    let weighttable_vec = create_weighttable(
        &tag_to_vector_map,
        &vector_of_tags,
        &configured_relative_tag_weights,
        configured_use_of_due_dates,
    );

    // last Vec will become the stats page
    // roll a assignment
    // first pick a tag to get an assignment from
    let tag_roll = roll_die(configured_relative_tag_weights.to_vec());
    let chosen_tag = &vector_of_tags[tag_roll];

    // then get the vector of assignments assigned to that tag
    let assignvector = tag_to_vector_map.get(chosen_tag).unwrap();
    // turn this into a pdf and roll an assignment
    let assignvector_pdf =
        turn_assignmentvector_into_pdf(&assignvector, configured_use_of_due_dates[tag_roll]);
    let chosen_assign = &assignvector[roll_die(assignvector_pdf)];

    // generate table string and current task string. this is for the tui
    let assign_string = taskvector_to_stringvect(chosen_assign);
    let string_alltask_vec = hashmap_to_taskvector(tag_to_vector_map, &vector_of_tags);
    return (assign_string, string_alltask_vec, weighttable_vec);
}

fn main() -> Result<(), Box<dyn Error>> {
    // instantiate the config file.
    let mut default_config = dirs::config_dir().unwrap();
    default_config.push("rusty-krab-manager");
    default_config.push("config.toml");

    default_files::create_default_files();

    let default_config = default_config.to_str().unwrap();

    // set config variables
    let (
        task_path,
        sound_path,
        tags,
        use_due_dates,
        initial_tag_weights,
        min_break_time,
        max_break_time,
        task_time,
        maxno_min_breaks,
    ) = settings_util::readin_settings(default_config)?;

    // initialize audio sink
    let sink = sound_utils::initialize_audio_sink();

    // initialize tag counter
    let mut tag_ctr = get_tag_counter_hashmap(&tags);

    // update tag weights if needed

    // Choose initial task
    let (curr_task, items_to_list, weighttable_vec) =
        choose_task(&task_path, &tags, &initial_tag_weights, &use_due_dates);

    // Terminal initialization for UI
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();
    let mut app = App::new();
    app.completed = convert_hashmap_to_tuplevector(&tag_ctr, &tags);
    app.current_task = curr_task;
    app.items = items_to_list;

    // Initialize starting parameters
    let mut min_break_ctr = 0;
    let mut its_task_time = true;
    let mut its_min_break_time = false;
    let mut its_max_break_time = false;
    let mut curr_screen = String::from("tasks");

    // create help table and flag
    let mut help_table = HelpTable::new();
    let mut weight_table = WeightTable::new(weighttable_vec);

    // Enter into UI drawing infinite loop
    loop {
        terminal.draw(|mut f| match curr_screen.as_str() {
            "help" => {
                let rects = Layout::default()
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .margin(2)
                    .split(f.size());
                draw_help(&mut f, &mut help_table, rects[0]);
            }
            "stats" => {
                let rects = Layout::default()
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .margin(2)
                    .split(f.size());
                draw_weights(&mut f, &mut weight_table, rects[0]);
            }
            _ => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Percentage(30),
                            Constraint::Percentage(50),
                            Constraint::Percentage(20),
                        ]
                        .as_ref(),
                    )
                    .split(f.size());
                let mini_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(75), Constraint::Percentage(25)].as_ref())
                    .split(chunks[0]);
                draw_gauge(&mut f, &app, chunks[2]);
                draw_task_table(&mut f, &app, chunks[1]);
                draw_current_task(&mut f, &app, mini_chunks[0]);
                draw_tag_counter(&mut f, &app, mini_chunks[1]);
            }
        })?;

        // keybindings
        match events.next()? {
            Event::Input(input) => match input {
                // denote the currently selected task as complete and reroll a new one
                Key::Char('c') => {
                    if its_task_time && !app.paused {
                        let mut fin_task_tag = app.current_task[0].clone();
                        fin_task_tag.pop();
                        *tag_ctr.get_mut(&fin_task_tag).unwrap() += 1;
                        app.completed = convert_hashmap_to_tuplevector(&tag_ctr, &tags);
                        let (curr_task, items_to_list, weighttable_vec) =
                            choose_task(&task_path, &tags, &initial_tag_weights, &use_due_dates);
                        weight_table = WeightTable::new(weighttable_vec);
                        app.current_task = curr_task;
                        app.items = items_to_list;
                    } else {
                    }
                }

                // reroll the currently selected task without marking current task as complete
                Key::Char('r') => {
                    if its_task_time && !app.paused {
                        let (curr_task, items_to_list, weighttable_vec) =
                            choose_task(&task_path, &tags, &initial_tag_weights, &use_due_dates);
                        weight_table = WeightTable::new(weighttable_vec);
                        app.current_task = curr_task;
                        app.items = items_to_list;
                    } else {
                    }
                }

                // fast forward timer to the end
                Key::Char('f') => {
                    app.progress = 1.0;
                }

                // QUIT
                Key::Char('q') => {
                    break;
                }

                // pause rkm
                Key::Char('p') => {
                    if app.paused {
                        app.paused = false;
                        app.current_task.pop();
                    } else {
                        app.paused = true;
                        app.current_task.push("PAUSED".to_string());
                    }
                }

                // move cursor down or up on task table
                Key::Down | Key::Char('j') => match curr_screen.as_str() {
                    "help" => {
                        help_table.next();
                    }
                    "stats" => {
                        weight_table.next();
                    }
                    "tasks" => {
                        app.selected += 1;
                        if app.selected > app.items.len() - 1 {
                            app.selected = 0
                        }
                    }
                    _ => {}
                },

                Key::Up | Key::Char('k') => match curr_screen.as_str() {
                    "help" => {
                        help_table.previous();
                    }
                    "stats" => {
                        weight_table.previous();
                    }
                    "tasks" => {
                        if app.selected > 0 {
                            app.selected -= 1;
                        } else {
                            app.selected = app.items.len() - 1;
                        }
                    }
                    _ => {}
                },

                Key::Char('h') => match curr_screen.as_str() {
                    "help" => {
                        curr_screen = String::from("tasks");
                    }
                    _ => {
                        curr_screen = String::from("help");
                    }
                },

                // toggle stats rkm
                Key::Char('s') => match curr_screen.as_str() {
                    "stats" => {
                        curr_screen = String::from("tasks");
                    }
                    _ => {
                        curr_screen = String::from("stats");
                    }
                },

                _ => {}
            },

            // what is done every 250 ms?
            Event::Tick => {
                // if app is paused do nothing.
                if app.paused {
                } else if its_task_time {
                    // is it time for a task?

                    // is next break a long break?
                    if min_break_ctr == maxno_min_breaks {
                        its_max_break_time = app.update(task_time);
                    } else {
                        // otherwise have a min break
                        its_min_break_time = app.update(task_time);
                    }

                    // if task time is up. reset task time. increment counter
                    //  task tracker
                    if its_min_break_time || its_max_break_time {
                        let mut fin_task_tag = app.current_task[0].clone();
                        fin_task_tag.pop();
                        *tag_ctr.get_mut(&fin_task_tag).unwrap() += 1;
                        app.completed = convert_hashmap_to_tuplevector(&tag_ctr, &tags);
                        sound_utils::playsound(&sound_path, &sink)?;
                        its_task_time = false;
                    };

                // time for a small break?
                } else if its_min_break_time {
                    app.current_task = vec![String::from("TAKE A CHILL PILL\n")];
                    its_task_time = app.update(min_break_time);

                    // if small break over, reroll task
                    if its_task_time {
                        sound_utils::playsound(&sound_path, &sink)?;
                        min_break_ctr += 1;
                        let (curr_task, items_to_list, weighttable_vec) =
                            choose_task(&task_path, &tags, &initial_tag_weights, &use_due_dates);
                        weight_table = WeightTable::new(weighttable_vec);
                        app.current_task = curr_task;
                        app.items = items_to_list;
                        its_min_break_time = false;
                    }

                // time for big break?
                } else if its_max_break_time {
                    app.current_task = vec![String::from("TAKE A LOONG CHILL PILL\n")];
                    its_task_time = app.update(max_break_time);

                    // if big break over, reroll task
                    if its_task_time {
                        sound_utils::playsound(&sound_path, &sink)?;
                        min_break_ctr = 0;
                        let (curr_task, items_to_list, weighttable_vec) =
                            choose_task(&task_path, &tags, &initial_tag_weights, &use_due_dates);
                        weight_table = WeightTable::new(weighttable_vec);
                        app.current_task = curr_task;
                        app.items = items_to_list;
                        its_max_break_time = false;
                    }
                }
            }
        };
    }
    Ok(())
}
