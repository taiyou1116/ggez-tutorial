use oorandom::Rand32;

use ggez::{
    event, graphics,
    input::keyboard::{KeyCode, KeyInput},
    Context, GameResult,
};

use std::collections::VecDeque;

const GRID_SIZE: (i16, i16) = (40, 30);
// Now we define the pixel size of each tile, which we make 32x32 pixels.
const GRID_CELL_SIZE: (i16, i16) = (42, 42);

// Next we define how large we want our actual window to be by multiplying
// the components of our grid size by its corresponding pixel size.
const SCREEN_SIZE: (f32, f32) = (
    GRID_SIZE.0 as f32 * GRID_CELL_SIZE.0 as f32,
    GRID_SIZE.1 as f32 * GRID_CELL_SIZE.1 as f32,
);

// 1秒間にupdateが呼ばれる回数
const DESIRED_FPS: u32 = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct GridPosition {
    x: i16,
    y: i16,
}

impl GridPosition {
    // x, yあわせてGridPositionとする
    pub fn new(x: i16, y: i16) -> Self {
        GridPosition { x, y }
    }

    // グリッド範囲内のランダムな位置を取得
    pub fn random(rng: &mut Rand32, max_x: i16, max_y: i16) -> Self {
        // GridPositionの型に合わせる
        (
            rng.rand_range(0..(max_x as u32)) as i16,
            rng.rand_range(0..(max_y as u32)) as i16,
        )
            .into()
    }

    // 受け取ったDirectionをGridPositionの座標に変換
    pub fn new_from_move(pos: GridPosition, dir: Direction) -> Self {
        match dir {
            Direction::Up => GridPosition::new(pos.x, (pos.y - 1).rem_euclid(GRID_SIZE.1)),
            Direction::Down => GridPosition::new(pos.x, (pos.y + 1).rem_euclid(GRID_SIZE.1)),
            Direction::Left => GridPosition::new((pos.x - 1).rem_euclid(GRID_SIZE.0), pos.y),
            Direction::Right => GridPosition::new((pos.x + 1).rem_euclid(GRID_SIZE.0), pos.y),
        }
    }
}

/// We implement the `From` trait, which in this case allows us to convert easily between
/// a `GridPosition` and a ggez `graphics::Rect` which fills that grid cell.
/// Now we can just call `.into()` on a `GridPosition` where we want a
/// `Rect` that represents that grid cell.
impl From<GridPosition> for graphics::Rect {
    fn from(pos: GridPosition) -> Self {
        graphics::Rect::new_i32(
            pos.x as i32 * GRID_CELL_SIZE.0 as i32,
            pos.y as i32 * GRID_CELL_SIZE.1 as i32,
            GRID_CELL_SIZE.0 as i32,
            GRID_CELL_SIZE.1 as i32,
        )
    }
}

/// And here we implement `From` again to allow us to easily convert between
/// `(i16, i16)` and a `GridPosition`.
impl From<(i16, i16)> for GridPosition {
    fn from(pos: (i16, i16)) -> Self {
        GridPosition { x: pos.0, y: pos.1 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    // 受け取ったDirectionを逆に変換
    pub fn inverse(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    // keycodeを受け取ったらSomeを返す
    pub fn from_keycode(key: KeyCode) -> Option<Direction> {
        match key {
            KeyCode::Up => Some(Direction::Up),
            KeyCode::Down => Some(Direction::Down),
            KeyCode::Left => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            _ => None,
        }
    }
}

/// This is mostly just a semantic abstraction over a `GridPosition` to represent
/// a segment of the snake. It could be useful to, say, have each segment contain its
/// own color or something similar. This is an exercise left up to the reader ;)
#[derive(Clone, Copy, Debug)]
struct Segment {
    pos: GridPosition,
}

impl Segment {
    pub fn new(pos: GridPosition) -> Self {
        Segment { pos }
    }
}

struct Food {
    pos: GridPosition,
}

impl Food {
    pub fn new(pos: GridPosition) -> Self {
        Food { pos }
    }

    // foodを描画する
    fn draw(&self, canvas: &mut graphics::Canvas) {
        // ブルー
        let color = [0.0, 0.0, 1.0, 1.0];

        // 四角形で描画
        canvas.draw(
            &graphics::Quad,
            graphics::DrawParam::new()
                .dest_rect(self.pos.into())
                .color(color),
        );
    }
}

// 食べたもの(自分かえさか)
#[derive(Clone, Copy, Debug)]
enum Ate {
    Itself,
    Food,
}

// スネーク
struct Snake {
    // 頭
    head: Segment,
    // 現在の方向
    dir: Direction,
    // 体
    body: VecDeque<Segment>,
    // 最後になんの餌を食ったか
    ate: Option<Ate>,
    // 最後の更新された方向
    last_update_dir: Direction,
    // 次のupdateで更新される方向(キー入力を保持)
    next_dir: Option<Direction>,
}

impl Snake {
    pub fn new(pos: GridPosition) -> Self {
        let mut body = VecDeque::new();
        // bosy要素を末尾に追加
        body.push_back(Segment::new((pos.x - 1, pos.y).into()));
        Snake {
            head: Segment::new(pos),
            dir: Direction::Right,
            last_update_dir: Direction::Right,
            body,
            ate: None,
            next_dir: None,
        }
    }

    // ヘッドの位置にfoodがあったらtrue
    fn eats(&self, food: &Food) -> bool {
        self.head.pos == food.pos
    }

    // ヘッドの位置がbodyのどこかと同じ位置にあったらtrue
    fn eats_self(&self) -> bool {
        for seg in &self.body {
            if self.head.pos == seg.pos {
                return true;
            }
        }
        false
    }

    fn update(&mut self, food: &Food) {
        // nextdirに新しく値が入った時
        if self.last_update_dir == self.dir && self.next_dir.is_some() {
            // 進行方向をnextdir, nextdirをNoneに
            self.dir = self.next_dir.unwrap();
            self.next_dir = None;
        }
        // 新しいヘッドの位置に今のヘッド位置 + 方向
        let new_head_pos = GridPosition::new_from_move(self.head.pos, self.dir);
        // ヘッド位置更新
        let new_head = Segment::new(new_head_pos);
        // bodyの先頭にヘッドを追加
        self.body.push_front(self.head);
        // headにnew_headを格納
        self.head = new_head;
        // 何か食べているかの判定
        if self.eats_self() {
            self.ate = Some(Ate::Itself);
        } else if self.eats(food) {
            self.ate = Some(Ate::Food);
        } else {
            self.ate = None;
        }
        // 何も食べていない場合は末尾のbodyを削除
        if self.ate.is_none() {
            self.body.pop_back();
        }
        // last_update_dirにdirを格納
        self.last_update_dir = self.dir;
    }

    // スネークを描画
    fn draw(&self, canvas: &mut graphics::Canvas) {
        for seg in &self.body {
            // body分描画
            canvas.draw(
                &graphics::Quad,
                graphics::DrawParam::new()
                    .dest_rect(seg.pos.into())
                    .color([0.3, 0.3, 0.0, 1.0]),
            );
        }
        // head描画
        canvas.draw(
            &graphics::Quad,
            graphics::DrawParam::new()
                .dest_rect(self.head.pos.into())
                .color([1.0, 0.5, 0.0, 1.0]),
        );
    }
}

// game内の全ての状態を管理
struct GameState {
    snake: Snake,
    food: Food,
    gameover: bool,
    rng: Rand32,
}

// newでGameStateのインスタンス(ゲームの初期状態)を作成
impl GameState {
    pub fn new() -> Self {
        // GRID_SIZE -> (30, 20)
        // 画面の横4/1, 高さ半分のところからスタート
        let snake_pos = (GRID_SIZE.0 / 4, GRID_SIZE.1 / 2).into();
        // u8型の配列の値それぞれにランダムな値を格納しu64に変換
        let mut seed: [u8; 8] = [0; 8];
        getrandom::getrandom(&mut seed[..]).expect("Could not create RNG seed");
        let mut rng = Rand32::new(u64::from_ne_bytes(seed));
        // Then we choose a random place to put our piece of food using the helper we made
        // earlier.
        let food_pos = GridPosition::random(&mut rng, GRID_SIZE.0, GRID_SIZE.1);

        GameState {
            snake: Snake::new(snake_pos),
            food: Food::new(food_pos),
            gameover: false,
            rng,
        }
    }
}

// EventHandlerトレイトで状態の更新を行う(update, draw)
impl event::EventHandler<ggez::GameError> for GameState {
    // drawよりも先に呼ばれる
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        while ctx.time.check_update_time(DESIRED_FPS) {
            // ゲームが続いていたら
            if !self.gameover {
                // ランダムフードの位置に蛇がいけば
                self.snake.update(&self.food);
                // 蛇が何か食った場合
                if let Some(ate) = self.snake.ate {
                    // If it did, we want to know what it ate.
                    match ate {
                        // foodだったら、新しくfoodをランダムな位置に追加
                        Ate::Food => {
                            let new_food_pos =
                                GridPosition::random(&mut self.rng, GRID_SIZE.0, GRID_SIZE.1);
                            self.food.pos = new_food_pos;
                        }
                        // bodyだったらgameover
                        Ate::Itself => {
                            self.gameover = true;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 描画
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // canvasインスタンスを作成、描画
        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::from([0.0, 0.0, 0.0, 0.0]));

        // snakeとfoodを描画
        self.snake.draw(&mut canvas);
        self.food.draw(&mut canvas);

        // 実際に描画
        canvas.finish(ctx)?;

        // 次のupdateまで他スレッドも実行
        ggez::timer::yield_now();

        Ok(())
    }

    /// キーが押されたタイミングで呼ばれる
    fn key_down_event(&mut self, _ctx: &mut Context, input: KeyInput, _repeat: bool) -> GameResult {
        // key入力を受け取る
        if let Some(dir) = input.keycode.and_then(Direction::from_keycode) {
            // If it succeeds, we check if a new direction has already been set
            // and make sure the new direction is different then `snake.dir`
            if self.snake.dir != self.snake.last_update_dir && dir.inverse() != self.snake.dir {
                self.snake.next_dir = Some(dir);
            } else if dir.inverse() != self.snake.last_update_dir {
                // If no new direction has been set and the direction is not the inverse
                // of the `last_update_dir`, then set the snake's new direction to be the
                // direction the user pressed.
                self.snake.dir = dir;
            }
        }
        Ok(())
    }
}

fn main() -> GameResult {
    // Here we use a ContextBuilder to setup metadata about our game. First the title and author
    let (ctx, events_loop) = ggez::ContextBuilder::new("snake", "Gray Olson")
        // Next we set up the window. This title will be displayed in the title bar of the window.
        .window_setup(ggez::conf::WindowSetup::default().title("Snake!"))
        // Now we get to set the size of the window, which we use our SCREEN_SIZE constant from earlier to help with
        .window_mode(ggez::conf::WindowMode::default().dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1))
        // And finally we attempt to build the context and create the window. If it fails, we panic with the message
        // "Failed to build ggez context"
        .build()?;

    // Next we create a new instance of our GameState struct, which implements EventHandler
    let state = GameState::new();
    // And finally we actually run our game, passing in our context and state.
    event::run(ctx, events_loop, state)
}
