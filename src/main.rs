use ncurses::*;
use std::cmp;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, ErrorKind, Write};
use std::ops::{Add, Mul};
use std::process;

const REGULAR_PAIR: i16 = 0;
const HIGHLIGHT_PAIR: i16 = 1;

/// Binds together data and behaviour for list manipulations
struct TaskList {
    pub list: Vec<String>,
    pub curr: usize,
}

impl TaskList {
    fn new(list: Vec<String>, curr: usize) -> Self {
        Self { list, curr }
    }

    /// Moves current list item one line up
    fn drag_up(&mut self) {
        if self.curr > 0 {
            self.list.swap(self.curr, self.curr - 1);
            self.curr -= 1;
        }
    }

    /// Moves current list item one line down
    fn drag_down(&mut self) {
        if self.curr + 1 < self.list.len() {
            self.list.swap(self.curr, self.curr + 1);
            self.curr += 1;
        }
    }

    /// Navigate to previous list item
    fn nav_up(&mut self) {
        if self.curr > 0 {
            self.curr -= 1;
        }
    }

    /// Navigate to next list item
    fn nav_down(&mut self) {
        if self.curr + 1 < self.list.len() {
            self.curr += 1;
        }
    }

    /// Navigate to the beginning of the list
    fn nav_first(&mut self) {
        if self.curr > 0 {
            self.curr = 0;
        }
    }

    /// Navigate to the end of the list
    fn nav_last(&mut self) {
        if !self.list.is_empty() {
            self.curr = self.list.len() - 1;
        }
    }

    /// Erases currently selected list item
    fn purge_curr(&mut self) {
        if self.curr < self.list.len() {
            self.list.remove(self.curr);
            if !self.list.is_empty() && self.curr >= self.list.len() {
                self.curr -= 1;
            }
        }
    }
}

/// Represents state of task lists
struct TaskManager {
    /// Indicates an active task list (panel)
    pub panel: Status,
    pub todos: TaskList,
    pub dones: TaskList,
}

impl TaskManager {
    fn new(panel: Status, todos: TaskList, dones: TaskList) -> Self {
        Self {
            panel,
            todos,
            dones,
        }
    }

    /// Returns an active task list (one which currently have focus)
    fn active(&mut self) -> &mut TaskList {
        match self.panel {
            Status::Todo => &mut self.todos,
            Status::Done => &mut self.dones,
        }
    }

    /// Returns an inactive task list (without focus)
    fn inactive(&mut self) -> &mut TaskList {
        match self.panel {
            Status::Done => &mut self.todos,
            Status::Todo => &mut self.dones,
        }
    }

    /// Moves selected list item from active to inactive panel (task list)
    fn transfer(&mut self) {
        let curr = self.active().curr;
        if curr < self.active().list.len() {
            let removed_elem = self.active().list.remove(curr);
            self.inactive().list.push(removed_elem);
            if curr >= self.active().list.len() && !self.active().list.is_empty() {
                self.active().curr = self.active().list.len() - 1;
            }
        }
    }
}

#[derive(Default, Copy, Clone)]
struct Vec2 {
    x: i32,
    y: i32,
}

impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Mul for Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl Vec2 {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

enum LayoutKind {
    Vert,
    Horz,
}

struct Layout {
    kind: LayoutKind,
    pos: Vec2,
    size: Vec2,
}

impl Layout {
    fn available_pos(&self) -> Vec2 {
        use LayoutKind::*;
        match self.kind {
            Horz => self.pos + self.size * Vec2::new(1, 0),
            Vert => self.pos + self.size * Vec2::new(0, 1),
        }
    }

    fn add_widget(&mut self, size: Vec2) {
        use LayoutKind::*;
        match self.kind {
            Horz => {
                self.size.x += size.x;
                self.size.y = cmp::max(self.size.y, size.y);
            }
            Vert => {
                self.size.x = cmp::max(self.size.x, size.x);
                self.size.y += size.y;
            }
        }
    }
}

#[derive(Default)]
struct Ui {
    layouts: Vec<Layout>,
}

impl Ui {
    fn begin(&mut self, pos: Vec2, kind: LayoutKind) {
        assert!(self.layouts.is_empty());
        self.layouts.push(Layout {
            kind,
            pos,
            size: Vec2::new(0, 0),
        })
    }

    fn begin_layout(&mut self, kind: LayoutKind) {
        let layout = self
            .layouts
            .last()
            .expect("Can't create a layout outside of Ui::begin() and Ui::end()");
        let pos = layout.available_pos();
        self.layouts.push(Layout {
            kind,
            pos,
            size: Vec2::new(0, 0),
        });
    }

    fn end_layout(&mut self) {
        let layout = self
            .layouts
            .pop()
            .expect("Unbalanced Ui::begin_layout() and Ui::end_layout() calls.");
        self.layouts
            .last_mut()
            .expect("Unbalanced Ui::begin_layout() and Ui::end_layout() calls.")
            .add_widget(layout.size);
    }

    fn label_fixed_width(&mut self, text: &str, width: i32, pair: i16) {
        // TODO(#17): Ui::label_fixed_width() does not elide the text when width < text.len()
        let layout = self
            .layouts
            .last_mut()
            .expect("Trying to render label outside of any layout");
        let pos = layout.available_pos();

        mv(pos.y, pos.x);
        attron(COLOR_PAIR(pair));
        addstr(text);
        attroff(COLOR_PAIR(pair));

        layout.add_widget(Vec2::new(width, 1));
    }

    #[allow(dead_code)]
    fn label(&mut self, text: &str, pair: i16) {
        self.label_fixed_width(text, text.len() as i32, pair);
    }

    fn end(&mut self) {
        self.layouts
            .pop()
            .expect("Unbalanced Ui::begin() and Ui::end() calls.");
    }
}

#[derive(Debug, PartialEq)]
enum Status {
    Todo,
    Done,
}

impl Status {
    fn toggle(&self) -> Self {
        match self {
            Status::Todo => Status::Done,
            Status::Done => Status::Todo,
        }
    }
}

fn parse_item(line: &str) -> Option<(Status, &str)> {
    let todo_item = line
        .strip_prefix("TODO: ")
        .map(|title| (Status::Todo, title));
    let done_item = line
        .strip_prefix("DONE: ")
        .map(|title| (Status::Done, title));
    todo_item.or(done_item)
}

fn load_state(todos: &mut Vec<String>, dones: &mut Vec<String>, file_path: &str) -> io::Result<()> {
    let file = File::open(file_path)?;
    for (index, line) in io::BufReader::new(file).lines().enumerate() {
        match parse_item(&line?) {
            Some((Status::Todo, title)) => todos.push(title.to_string()),
            Some((Status::Done, title)) => dones.push(title.to_string()),
            None => {
                eprintln!("{}:{}: ERROR: ill-formed item line", file_path, index + 1);
                process::exit(1);
            }
        }
    }
    Ok(())
}

fn save_state(todos: &[String], dones: &[String], file_path: &str) {
    let mut file = File::create(file_path).unwrap();
    for todo in todos.iter() {
        writeln!(file, "TODO: {}", todo).unwrap();
    }
    for done in dones.iter() {
        writeln!(file, "DONE: {}", done).unwrap();
    }
}

// TODO(#2): add new items to TODO
// TODO(#4): edit the items
// TODO(#5): keep track of date when the item was DONE
// TODO(#6): undo system
// TODO(#12): save the state on SIGINT

fn main() {
    let mut args = env::args();
    args.next().unwrap();

    let file_path = match args.next() {
        Some(file_path) => file_path,
        None => {
            eprintln!("Usage: todo-rs <file-path>");
            eprintln!("ERROR: file path is not provided");
            process::exit(1);
        }
    };

    let mut todos = Vec::<String>::new();
    let todo_curr: usize = 0;
    let mut dones = Vec::<String>::new();
    let done_curr: usize = 0;

    let mut notification: String;

    match load_state(&mut todos, &mut dones, &file_path) {
        Ok(()) => notification = format!("Loaded file {}", file_path),
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                notification = format!("New file {}", file_path)
            } else {
                panic!(
                    "Could not load state from file `{}`: {:?}",
                    file_path, error
                );
            }
        }
    };

    initscr();
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    start_color();
    init_pair(REGULAR_PAIR, COLOR_WHITE, COLOR_BLACK);
    init_pair(HIGHLIGHT_PAIR, COLOR_BLACK, COLOR_WHITE);

    let mut quit = false;
    let panel = Status::Todo;

    let mut ui = Ui::default();
    let mut tasks = TaskManager::new(
        panel,
        TaskList::new(todos, todo_curr),
        TaskList::new(dones, done_curr),
    );
    while !quit {
        erase();

        let mut x = 0;
        let mut y = 0;
        getmaxyx(stdscr(), &mut y, &mut x);

        ui.begin(Vec2::new(0, 0), LayoutKind::Vert);
        {
            ui.label_fixed_width(&notification, x, REGULAR_PAIR);
            notification.clear();
            ui.label_fixed_width("", x, REGULAR_PAIR);

            ui.begin_layout(LayoutKind::Horz);
            {
                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label_fixed_width(
                        "TODO",
                        x / 2,
                        if tasks.panel == Status::Todo {
                            HIGHLIGHT_PAIR
                        } else {
                            REGULAR_PAIR
                        },
                    );
                    for (index, todo) in tasks.todos.list.iter().enumerate() {
                        ui.label_fixed_width(
                            &format!("- [ ] {}", todo),
                            x / 2,
                            if index == tasks.todos.curr && tasks.panel == Status::Todo {
                                HIGHLIGHT_PAIR
                            } else {
                                REGULAR_PAIR
                            },
                        );
                    }
                }
                ui.end_layout();

                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label_fixed_width(
                        "DONE",
                        x / 2,
                        if tasks.panel == Status::Done {
                            HIGHLIGHT_PAIR
                        } else {
                            REGULAR_PAIR
                        },
                    );
                    for (index, done) in tasks.dones.list.iter().enumerate() {
                        ui.label_fixed_width(
                            &format!("- [x] {}", done),
                            x / 2,
                            if index == tasks.dones.curr && tasks.panel == Status::Done {
                                HIGHLIGHT_PAIR
                            } else {
                                REGULAR_PAIR
                            },
                        );
                    }
                }
                ui.end_layout();
            }
            ui.end_layout();
        }
        ui.end();

        refresh();

        let key = getch();
        match key as u8 as char {
            'q' => quit = true,
            'K' => tasks.active().drag_up(),
            'J' => tasks.active().drag_down(),
            'k' => tasks.active().nav_up(),
            'j' => tasks.active().nav_down(),
            'g' => tasks.active().nav_first(),
            'G' => tasks.active().nav_last(),
            'd' => tasks.active().purge_curr(),
            '\n' => {
                tasks.transfer();

                notification.push_str(match tasks.panel {
                    Status::Todo => "DONE!",
                    Status::Done => "No, not done yet...",
                });
            }
            '\t' | 'h' | 'l' => {
                tasks.panel = tasks.panel.toggle();
            }
            _ => {
                // todos.push(format!("{}", key));
            }
        }
    }

    endwin();

    save_state(&tasks.todos.list, &tasks.dones.list, &file_path);
    println!("Saved state to {}", file_path);
}
