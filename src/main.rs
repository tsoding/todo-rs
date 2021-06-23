use ncurses::*;
use std::cmp;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, ErrorKind, Write};
use std::ops::{Add, Mul};
use std::process;

const REGULAR_PAIR: i16 = 0;
const HIGHLIGHT_PAIR: i16 = 1;

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
    Deleted,
}

impl Status {
    fn toggle(&self) -> Self {
        match self {
            Status::Todo => Status::Done,
            Status::Done => Status::Todo,
            _ => unreachable!(),
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
    let deleted_item = line
        .strip_prefix("DELETED: ")
        .map(|title| (Status::Deleted, title));

    todo_item.or(done_item).or(deleted_item)
}

fn list_drag_up(list: &mut [String], list_curr: &mut usize) {
    if *list_curr > 0 {
        list.swap(*list_curr, *list_curr - 1);
        *list_curr -= 1;
    }
}

fn list_drag_down(list: &mut [String], list_curr: &mut usize) {
    if *list_curr + 1 < list.len() {
        list.swap(*list_curr, *list_curr + 1);
        *list_curr += 1;
    }
}

fn list_up(list_curr: &mut usize) {
    if *list_curr > 0 {
        *list_curr -= 1;
    }
}

fn list_down(list: &[String], list_curr: &mut usize) {
    if *list_curr + 1 < list.len() {
        *list_curr += 1;
    }
}

fn list_first(list_curr: &mut usize) {
    if *list_curr > 0 {
        *list_curr = 0;
    }
}

fn list_last(list: &[String], list_curr: &mut usize) {
    if !list.is_empty() {
        *list_curr = list.len() - 1;
    }
}

fn list_transfer(
    list_dst: &mut Vec<String>,
    list_src: &mut Vec<String>,
    list_src_curr: &mut usize,
) {
    if *list_src_curr < list_src.len() {
        list_dst.push(list_src.remove(*list_src_curr));
        if *list_src_curr >= list_src.len() && !list_src.is_empty() {
            *list_src_curr = list_src.len() - 1;
        }
    }
}

fn delete(list: &mut Vec<String>, list_curr: &mut usize, deleteds: &mut Vec<String>) {
    list_transfer(deleteds, list, list_curr)
}

fn load_state(
    todos: &mut Vec<String>,
    dones: &mut Vec<String>,
    deleteds: &mut Vec<String>,
    file_path: &str,
) -> io::Result<()> {
    let file = File::open(file_path)?;
    for (index, line) in io::BufReader::new(file).lines().enumerate() {
        match parse_item(&line?) {
            Some((Status::Todo, title)) => todos.push(title.to_string()),
            Some((Status::Done, title)) => dones.push(title.to_string()),
            Some((Status::Deleted, title)) => deleteds.push(title.to_string()),
            None => {
                eprintln!("{}:{}: ERROR: ill-formed item line", file_path, index + 1);
                process::exit(1);
            }
        }
    }
    Ok(())
}

fn save_state(todos: &[String], dones: &[String], deleteds: &[String], file_path: &str) {
    let mut file = File::create(file_path).unwrap();
    for todo in todos.iter() {
        writeln!(file, "TODO: {}", todo).unwrap();
    }
    for done in dones.iter() {
        writeln!(file, "DONE: {}", done).unwrap();
    }
    for deleted in deleteds.iter() {
        writeln!(file, "DELETED: {}", deleted).unwrap()
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
    let mut todo_curr: usize = 0;
    let mut dones = Vec::<String>::new();
    let mut done_curr: usize = 0;
    let mut deleteds = Vec::<String>::new();
    let mut deleteds_curr: usize = 0;

    let mut notification: String;

    match load_state(&mut todos, &mut dones, &mut deleteds, &file_path) {
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
    let mut panel = Status::Todo;

    let mut ui = Ui::default();
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
                        if panel == Status::Todo {
                            HIGHLIGHT_PAIR
                        } else {
                            REGULAR_PAIR
                        },
                    );
                    for (index, todo) in todos.iter().enumerate() {
                        ui.label_fixed_width(
                            &format!("- [ ] {}", todo),
                            x / 2,
                            if index == todo_curr && panel == Status::Todo {
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
                        if panel == Status::Done {
                            HIGHLIGHT_PAIR
                        } else {
                            REGULAR_PAIR
                        },
                    );
                    for (index, done) in dones.iter().enumerate() {
                        ui.label_fixed_width(
                            &format!("- [x] {}", done),
                            x / 2,
                            if index == done_curr && panel == Status::Done {
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
            'd' => match panel {
                Status::Todo => delete(&mut todos, &mut todo_curr, &mut deleteds),
                Status::Done => delete(&mut dones, &mut done_curr, &mut deleteds),
                _ => (),
            },
            'K' => match panel {
                Status::Todo => list_drag_up(&mut todos, &mut todo_curr),
                Status::Done => list_drag_up(&mut dones, &mut done_curr),
                _ => (),
            },
            'J' => match panel {
                Status::Todo => list_drag_down(&mut todos, &mut todo_curr),
                Status::Done => list_drag_down(&mut dones, &mut done_curr),
                _ => (),
            },
            'k' => match panel {
                Status::Todo => list_up(&mut todo_curr),
                Status::Done => list_up(&mut done_curr),
                _ => (),
            },
            'j' => match panel {
                Status::Todo => list_down(&todos, &mut todo_curr),
                Status::Done => list_down(&dones, &mut done_curr),
                _ => (),
            },
            'g' => match panel {
                Status::Todo => list_first(&mut todo_curr),
                Status::Done => list_first(&mut done_curr),
                _ => (),
            },
            'G' => match panel {
                Status::Todo => list_last(&todos, &mut todo_curr),
                Status::Done => list_last(&dones, &mut done_curr),
                _ => (),
            },
            '\n' => match panel {
                Status::Todo => {
                    list_transfer(&mut dones, &mut todos, &mut todo_curr);
                    notification.push_str("DONE!")
                }
                Status::Done => {
                    list_transfer(&mut todos, &mut dones, &mut done_curr);
                    notification.push_str("No, not done yet...")
                }
                _ => (),
            },
            '\t' => {
                panel = panel.toggle();
            }
            _ => {
                // todos.push(format!("{}", key));
            }
        }
    }

    endwin();

    save_state(&todos, &dones, &deleteds, &file_path);
    println!("Saved state to {}", file_path);
}
