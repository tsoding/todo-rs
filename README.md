# todo-rs

Simple Interactive Terminal Todo App in Rust. This is meant to be an experimental playground for testing ideas on Immediate TUIs.

![thumbnail](thumbnail.png)

## Quick Start

```console
$ cargo run TODO
```

## Controls

|Keys|Description|
|---|---|
|<kbd>k</kbd>, <kbd>j</kbd>|Move cursor up and down|
|<kbd>Shift+K</kbd>, <kbd>Shift+J</kbd>|Drag the current item up and down|
|<kbd>g</kbd>, <kbd>G</kbd> | Jump to the start, end of the current item list|
|<kbd>r</kbd>|Rename the current item|
|<kbd>i</kbd>|Insert a new item|
|<kbd>d</kbd>|Delete the current list item|
|<kbd>q</kbd>|Quit|
|<kbd>TAB</kbd>|Switch between the TODO and DONE panels|
|<kbd>Enter</kbd>|Perform an action on the highlighted UI element|
