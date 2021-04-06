use std::process::exit;
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;
use gtk::prelude::*;
use gio::prelude::*;
use gtk::{Application, ApplicationWindow, Switch, Grid, MessageDialog, ButtonsType, DialogFlags, MessageType, Window};
use rand::Rng;

fn main() {
    //Init GTK
    let application = Application::new(
        Some("com.marekkon5.gtksnake"),
        Default::default()
    ).expect("Failed to initialize GTK application");

    let width = 14;
    let height = 10;
    let delay = 250;
    
    application.connect_activate(move |app| {
        //Create window
        let window = ApplicationWindow::new(app);
        window.set_title("GTK Snake");

        //Generate grid
        let mut switches = vec![];
        let grid = Grid::new();
        grid.set_row_spacing(8);
        grid.set_column_spacing(8);
        for y in 0..height {
            let mut row = vec![];
            for x in 0..width {
                let switch = Switch::new();
                grid.attach(&switch, x, y, 1, 1);
                row.push(switch);
            }
            switches.push(row);
        }        
        window.add(&grid);

        //Keypress
        let (tx1, rx1) = sync_channel(100);
        window.connect("key_press_event", false, move |values| {
            let raw_event = &values[1].get::<gdk::Event>().unwrap().unwrap();
            match raw_event.downcast_ref::<gdk::EventKey>() {
                Some(event) => {
                    let key = std::char::from_u32(*event.get_keyval()).unwrap_or('\0').to_lowercase();
                    //Handle key
                    tx1.send(key.to_string().chars().next().unwrap()).ok();
                }
                None => {}
            };

            //Return something
            Some(glib::value::Value::from_type(glib::types::Type::Bool))
        }).unwrap();

        window.show_all();

        //Create thread
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        thread::spawn(move || {
            let mut state = GameState::new(width as usize, height as usize);
            let mut player = Player::new(width/2, height/2);
            let mut point = Coordinate::random(width, height);

            loop {
                //Move
                for key in rx1.try_iter() {
                    player.handle_keypress(key);
                }
                player.do_move(width, height);

                //Got point
                if player.coordinate == point {
                    player.extend();
                    state.score += 1;
                    point = Coordinate::random(width, height);
                }

                //Colision check
                if player.body.iter().any(|b| b == &player.coordinate) {
                    state.dead = true;
                    break;
                }

                //Draw grid
                state.grid.clear();
                state.grid.set(&vec![player.coordinate.clone(), point.clone()], true);
                state.grid.set(&player.body, true);

                //Update UI
                tx.send(state.clone()).ok();
                thread::sleep(Duration::from_millis(delay));
            }
            //Died
            tx.send(state).unwrap();
        });
        //Receive messages
        rx.attach(None, move |game| {
            //Update grid
            for y in 0..game.grid.height {
                for x in 0..game.grid.width {
                    if switches[y][x].get_active() != game.grid.data[y][x] {
                        switches[y][x].activate();
                    }
                }
            }

            //Score
            window.set_title(&format!("GTK Snake | Score: {}", game.score));

            //Game over
            if game.dead {
                MessageDialog::new(
                    None::<&Window>, 
                    DialogFlags::empty(), 
                    MessageType::Error,
                    ButtonsType::Ok, 
                    &format!("You lost! Score: {}", game.score)
                ).run();
                exit(0);
            }

            glib::Continue(true)
        });
    });

    application.run(&[]);
}

#[derive(Debug, Clone)]
struct GameState {
    pub grid: GameGrid,
    pub score: i32,
    pub dead: bool
}

impl GameState {
    pub fn new(width: usize, height: usize) -> GameState {
        GameState {
            grid: GameGrid::new(width, height),
            score: 0,
            dead: false
        }
    }
}

#[derive(Debug, Clone)]
struct GameGrid {
    pub data: Vec<Vec<bool>>,
    pub width: usize,
    pub height: usize,
}

impl GameGrid {
    //Create empty grid
    pub fn new(width: usize, height: usize) -> GameGrid {
        GameGrid {
            width, 
            height,
            data: vec![vec![false; width]; height]
        }
    }

    //Clear grid
    pub fn clear(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.data[y][x] = false;
            }
        }
    }

    //Switch list of coordinates
    pub fn set(&mut self, coordinates: &Vec<Coordinate>, state: bool) {
        for c in coordinates {
            self.data[c.y as usize][c.x as usize] = state;
        }
    }
}

#[derive(Debug, Clone)]
struct Player {
    pub coordinate: Coordinate,
    pub body: Vec<Coordinate>,
    //x, y
    pub direction: [i8; 2],
    extend: bool
}

impl Player {
    pub fn new(x: i32, y: i32) -> Player {
        Player {
            coordinate: Coordinate::new(x, y),
            body: vec![],
            direction: [1, 0],
            extend: false
        }
    }

    //Move snake
    pub fn do_move(&mut self, width: i32, height: i32) {
        //Move body
        if !self.body.is_empty() || self.extend {
            if !self.extend {
                self.body.pop();
            } else {
                self.extend = false;
            }
            self.body.insert(0, self.coordinate.clone());
        }
        self.coordinate.x += self.direction[0] as i32;
        self.coordinate.y += self.direction[1] as i32;
        //Bounds
        if self.coordinate.x >= width {
            self.coordinate.x = 0;
        }
        if self.coordinate.x < 0 {
            self.coordinate.x = width - 1;
        }
        if self.coordinate.y >= height {
            self.coordinate.y = 0;
        }
        if self.coordinate.y < 0 {
            self.coordinate.y = height - 1;
        }
    }

    //Handle keyboard press
    pub fn handle_keypress(&mut self, key: char) {
        let old = self.direction.clone();
        match key {
            'w' => self.direction = [0, -1],
            'a' => self.direction = [-1, 0],
            's' => self.direction = [0, 1],
            'd' => self.direction = [1, 0],
            _ => {}
        }

        //Check for invalid move
        if !self.body.is_empty() && (
            (old[0] != 0 && old[0] * -1 == self.direction[0]) ||
            (old[1] != 0 && old[1] * -1 == self.direction[1])) {
            self.direction = old;
        }
    }

    //Extend
    pub fn extend(&mut self) {
        self.extend = true;
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Coordinate {
    pub x: i32,
    pub y: i32
}

impl Coordinate {
    pub fn new(x: i32, y: i32) -> Coordinate {
        Coordinate {x, y}
    }

    //Generate random coordinate
    pub fn random(max_x: i32, max_y: i32) -> Coordinate {
        let mut rng = rand::thread_rng();
        Coordinate {
            x: rng.gen_range(0..max_x),
            y: rng.gen_range(0..max_y)
        }
    }
}