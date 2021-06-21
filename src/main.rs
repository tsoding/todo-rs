use ncurses::*;

const REGULAR_PAIR: i16 = 0;
const HIGHLIGHT_PAIR: i16 = 1;

type Id = usize;

#[derive(Default)]
struct Ui {
    list_curr: Option<Id>,
    row: usize,
    col: usize,
}

impl Ui {
    fn begin(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }

    fn begin_list(&mut self, id: Id) {
        assert!(self.list_curr.is_none(), "Nested lists are not allowed!");
        self.list_curr = Some(id);
    }

    fn list_element(&mut self, label: &str, id: Id) -> bool {
        let id_curr = self
            .list_curr
            .expect("Not allowed to create list elements outside of lists");

        self.label(label, {
            if id_curr == id {
                HIGHLIGHT_PAIR
            } else {
                REGULAR_PAIR
            }
        });

        return false;
    }

    fn label(&mut self, text: &str, pair: i16) {
        mv(self.row as i32, self.col as i32);
        attron(COLOR_PAIR(pair));
        addstr(text);
        attroff(COLOR_PAIR(pair));
        self.row += 1;
    }

    fn end_list(&mut self) {
        self.list_curr = None;
    }

    fn end(&mut self) {}
}

enum Tab {
    Todo,
    Done,
}

impl Tab {
    fn toggle(&self) -> Self {
        match self {
            Tab::Todo => Tab::Done,
            Tab::Done => Tab::Todo,
        }
    }
}

fn list_up(list_curr: &mut usize) {
    if *list_curr > 0 {
        *list_curr -= 1;
    }
}

fn list_down(list: &Vec<String>, list_curr: &mut usize) {
    if *list_curr + 1 < list.len() {
        *list_curr += 1;
    }
}

fn list_transfer(list_dst: &mut Vec<String>, list_src: &mut Vec<String>, list_src_curr: &mut usize) {
    if *list_src_curr < list_src.len() {
        list_dst.push(list_src.remove(*list_src_curr));
        if *list_src_curr >= list_src.len() && list_src.len() > 0 {
            *list_src_curr = list_src.len() - 1;
        }
    }
}

// TODO(#1): persist the state of the application
// TODO(#2): add new items to TODO
// TODO(#3): delete items
// TODO(#4): edit the items
// TODO: keep track of date when the item was DONE
// TODO: undo system

fn main() {
    initscr();
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    start_color();
    init_pair(REGULAR_PAIR, COLOR_WHITE, COLOR_BLACK);
    init_pair(HIGHLIGHT_PAIR, COLOR_BLACK, COLOR_WHITE);

    let mut quit = false;
    let mut todos: Vec<String> = vec![
        "Write the todo app".to_string(),
        "Buy a bread".to_string(),
        "Make a cup of tea".to_string(),
    ];
    let mut todo_curr: usize = 0;
    let mut dones: Vec<String> = vec![
        "Start the stream".to_string(),
        "Have a breakfast".to_string(),
        "Make a cup of tea".to_string(),
    ];
    let mut done_curr: usize = 0;
    let mut tab = Tab::Todo;

    let mut ui = Ui::default();
    while !quit {
        erase();

        ui.begin(0, 0);
        {
            match tab {
                Tab::Todo => {
                    ui.label("[TODO] DONE ", REGULAR_PAIR);
                    ui.label("------------", REGULAR_PAIR);
                    ui.begin_list(todo_curr);
                    for (index, todo) in todos.iter().enumerate() {
                        ui.list_element(&format!("- [ ] {}", todo), index);
                    }
                    ui.end_list();
                }
                Tab::Done => {
                    ui.label(" TODO [DONE]", REGULAR_PAIR);
                    ui.label("------------", REGULAR_PAIR);
                    ui.begin_list(done_curr);
                    for (index, done) in dones.iter().enumerate() {
                        ui.list_element(&format!("- [x] {}", done), index);
                    }
                    ui.end_list();
                }
            }
        }
        ui.end();

        refresh();

        let key = getch();
        match key as u8 as char {
            'q' => quit = true,
            'w' => match tab {
                Tab::Todo => list_up(&mut todo_curr),
                Tab::Done => list_up(&mut done_curr),
            },
            's' => match tab {
                Tab::Todo => list_down(&todos, &mut todo_curr),
                Tab::Done => list_down(&dones, &mut done_curr),
            },
            '\n' => match tab {
                Tab::Todo => list_transfer(&mut dones, &mut todos, &mut todo_curr),
                Tab::Done => list_transfer(&mut todos, &mut dones, &mut done_curr),
            },
            '\t' => {
                tab = tab.toggle();
            }
            _ => {
                // todos.push(format!("{}", key));
            }
        }
    }

    endwin();
}
