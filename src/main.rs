use grid::*;

use sfml::graphics::*;
use sfml::system::*;
use sfml::window::*;

const SPEED_FACTOR: Time = Time::milliseconds(10);
const STATUS_BAR_HEIGHT: u32 = 40;
const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;
const CELL_ROWS: u32 = 32;
const CELL_COLS: u32 = 32;
const GROWTH_FACTOR: usize = 4;
const TIME_BETWEEN_STEPS: Time = Time::milliseconds(200);

fn main() {
    let mut window = RenderWindow::new(
        (WIDTH, HEIGHT + STATUS_BAR_HEIGHT),
        "Conway's game of life",
        Style::CLOSE,
        &Default::default(),
    );

    window.set_vertical_sync_enabled(true);

    let font_bytes = include_bytes!("../Hack NF.ttf");
    let mut font = unsafe { Font::from_memory(font_bytes).unwrap() };
    font.set_smooth(true);

    let mut text = Text::new("FPS", &font, 16);
    text.set_position((10.0, HEIGHT as f32 + 10.0));
    text.set_fill_color(Color::WHITE);

    let mut status_rect = RectangleShape::new();
    status_rect.set_outline_thickness(2.0);
    status_rect.set_outline_color(Color::WHITE);
    status_rect.set_fill_color(Color::BLACK);
    status_rect.set_position((0.0, HEIGHT as f32));
    status_rect.set_size((WIDTH as f32, STATUS_BAR_HEIGHT as f32));

    let mut state = GameState::new(CELL_ROWS, CELL_COLS);
    let mut clock = Clock::start();

    let mut msg = String::new();

    while window.is_open() {
        let dt = clock.restart();

        while let Some(event) = window.poll_event() {
            if let Some(m) = state.handle_event(&event) {
                msg = m;
            }

            if let Event::Closed = event {
                window.close();
            }
        }

        text.set_string(&format!(
            "FPS: {:.0}, {}, speed: {}ms, grid: {3}x{3}{4}",
            1.0 / dt.as_seconds(),
            if state.auto_play { "playing" } else { "paused" },
            state.time_bw_steps.as_milliseconds(),
            state.get_dimensions().0,
            if msg.is_empty() {
                "".to_string()
            } else {
                format!(", {msg}")
            }
        ));

        window.clear(Color::BLACK);
        state.draw(&mut window, dt);
        window.draw(&status_rect);
        window.draw(&text);
        window.display();
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum CellState {
    /// Cell is alive
    Alive,

    #[default]
    /// Cell is dead
    Dead,
}

impl CellState {
    pub fn is_alive(&self) -> bool {
        *self == Self::Alive
    }

    pub fn set_color<'a>(&self, rect: &mut RectangleShape<'a>) {
        static WHITE: Color = Color::rgb(50, 50, 50);
        static YELLOW: Color = Color::rgb(200, 200, 0);
        match self {
            Self::Alive => {
                rect.set_fill_color(YELLOW);
                rect.set_outline_color(Color::BLACK);
            }
            Self::Dead => {
                rect.set_fill_color(Color::BLACK);
                rect.set_outline_color(WHITE);
            }
        }
    }

    pub fn toggle(&mut self) {
        *self = match self {
            Self::Alive => Self::Dead,
            Self::Dead => Self::Alive,
        };
    }
}

struct GameState<'a> {
    grid: Grid<CellState>,
    drawing_rect: RectangleShape<'a>,

    button_pressed: bool,

    elapsed_time: Time,

    pub auto_play: bool,

    pub time_bw_steps: Time,

    // used for handling the cell toggle
    toggled_cell: (i32, i32),
}

impl<'a> GameState<'a> {
    pub fn new(rows: u32, cols: u32) -> Self {
        Self {
            time_bw_steps: TIME_BETWEEN_STEPS,
            auto_play: false,
            elapsed_time: Time::ZERO,
            toggled_cell: (-1, -1),
            button_pressed: false,
            grid: Grid::new(rows as _, cols as _),
            drawing_rect: {
                let mut rect = RectangleShape::new();
                rect.set_size(((WIDTH / rows) as f32, (HEIGHT / cols) as f32));
                rect.set_outline_thickness(0.5);
                rect
            },
        }
    }

    pub fn reset(&mut self) {
        self.grid
            .iter_mut()
            .for_each(|cell| *cell = CellState::Dead);
    }

    pub fn is_clear(&self) -> bool {
        self.grid.iter().all(|cell| *cell == CellState::Dead)
    }

    pub fn get_dimensions(&self) -> (usize, usize) {
        (self.grid.rows(), self.grid.cols())
    }

    /// Optionally a message that has to be displayed in the status bar
    pub fn handle_event(&mut self, event: &Event) -> Option<String> {
        match event {
            &Event::KeyPressed { code, shift, .. } => {
                match code {
                    Key::Space => self.step(),
                    Key::R => self.reset(),
                    Key::P => {
                        self.auto_play = !self.auto_play;
                        self.elapsed_time = Time::ZERO;
                    }

                    Key::Add if shift => self.time_bw_steps += SPEED_FACTOR,
                    Key::Subtract if shift => {
                        if self.time_bw_steps <= SPEED_FACTOR {
                            return Some(String::from("Cannot decrease further"));
                        }

                        self.time_bw_steps -= SPEED_FACTOR
                    }

                    Key::Add => {
                        if !self.is_clear() {
                            return Some(String::from("Reset the grid first"));
                        } else if self.grid.cols() == 40 {
                            return Some(String::from("Max grid size reached"));
                        }

                        self.grid = Grid::new(
                            self.grid.rows() + GROWTH_FACTOR,
                            self.grid.cols() + GROWTH_FACTOR,
                        );

                        self.drawing_rect.set_size((
                            WIDTH as f32 / self.grid.rows() as f32,
                            HEIGHT as f32 / self.grid.cols() as f32,
                        ));
                    }

                    Key::Subtract => {
                        if !self.is_clear() {
                            return Some(String::from("Reset the grid first"));
                        } else if self.grid.cols() == 4 {
                            return Some(String::from("Min grid size reached"));
                        }

                        self.grid = Grid::new(
                            self.grid.rows() - GROWTH_FACTOR,
                            self.grid.cols() - GROWTH_FACTOR,
                        );

                        self.drawing_rect.set_size((
                            WIDTH as f32 / self.grid.rows() as f32,
                            HEIGHT as f32 / self.grid.cols() as f32,
                        ));
                    }

                    _ => {}
                }

                return Some(String::new());
            }

            &Event::MouseButtonPressed { button, x, y }
                if matches![button, mouse::Button::Left] =>
            {
                self.button_pressed = true;
                self.toggle_cell(x, y);
                return Some(String::new());
            }

            &Event::MouseButtonReleased { button, .. } if matches![button, mouse::Button::Left] => {
                self.button_pressed = false;
                self.toggled_cell = (-1, -1);
            }

            &Event::MouseMoved { x, y } if self.button_pressed => self.toggle_cell(x, y),

            _ => {}
        }

        None
    }

    pub fn toggle_cell(&mut self, x: i32, y: i32) {
        let cell_width = WIDTH / self.grid.rows() as u32;
        let cell_height = HEIGHT / self.grid.cols() as u32;

        // column of the cell
        let col_idx = x as i32 / cell_width as i32;

        // row of the cell
        let row_idx = y as i32 / cell_height as i32;

        match &mut self.toggled_cell {
            (last_row, last_col) if *last_row != row_idx || *last_col != col_idx => {
                if let Some(cell) = self.grid.get_mut(row_idx as _, col_idx as _) {
                    cell.toggle();
                }

                (*last_row, *last_col) = (row_idx, col_idx);
            }

            _ => {}
        }
    }

    pub fn step(&mut self) {
        let mut new_grid = self.grid.clone();

        for row_idx in 0..self.grid.rows() {
            for col_idx in 0..self.grid.cols() {
                let n = self.get_num_alive_neighbours(row_idx, col_idx);
                match self.grid[row_idx][col_idx] {
                    CellState::Alive if n < 2 || n > 3 => {
                        new_grid[row_idx][col_idx] = CellState::Dead;
                    }

                    CellState::Dead if n == 3 => {
                        new_grid[row_idx][col_idx] = CellState::Alive;
                    }

                    _ => {}
                }
            }
        }

        self.grid = new_grid;
    }

    pub fn draw(&mut self, window: &mut RenderWindow, dt: Time) {
        if self.auto_play {
            self.elapsed_time += dt;

            if self.elapsed_time > self.time_bw_steps {
                self.elapsed_time %= self.time_bw_steps;
                self.step();
            }
        }

        let cell_width = WIDTH / self.grid.rows() as u32;
        let cell_height = HEIGHT / self.grid.cols() as u32;

        for row_idx in 0..self.grid.rows() {
            for (col_idx, cell) in self.grid.iter_row(row_idx).enumerate() {
                cell.set_color(&mut self.drawing_rect);

                self.drawing_rect.set_position((
                    col_idx as f32 * cell_width as f32,
                    row_idx as f32 * cell_height as f32,
                ));

                window.draw(&self.drawing_rect);
            }
        }
    }

    pub fn get_num_alive_neighbours(&self, row: usize, col: usize) -> usize {
        let mut n = 0;

        for dr in -1..=1 {
            for dc in -1..=1 {
                if dr == 0 && dc == 0 {
                    continue; // Skip the current cell
                }

                let neighbor_row = (row as i32).wrapping_add(dr);
                let neighbor_col = (col as i32).wrapping_add(dc);

                self.grid
                    .get(neighbor_row as _, neighbor_col as _)
                    .unwrap_or(&CellState::Dead)
                    .is_alive()
                    .then(|| {
                        n += 1;
                    });
            }
        }

        n
    }
}
