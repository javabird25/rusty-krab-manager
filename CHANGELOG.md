## 0.1.2
* bumped dirs to 3.0
* only allow 'c' and 'r' bindings to do things if app is not paused and is currently on a task
* hanging back on rodio 0.11 until [this](https://github.com/RustAudio/rodio/issues/290) is resolved
* add 'h' to toggle a help menu which describes keypresses
* ROUNDED CORNERS!!!

## 1.0.0
* increased the space for help page description
* bumped tui-rs to v.0.10 + made TagCtr use ITALICS
* added space between 'DO THIS SHIT' and curr. task
* made the tag counter *italics*
* added 'match' logic for **c r i s p** screen switching
* added 'match' logic for **c r i s p** tag weight decision 
* fixed bug in recomputing of tag weights (#19)
* moved update_tag_weights to assignment_utils
* added 's' to toggle a stats menu which describes calculated probability of a given task
* unified highlight symbols
