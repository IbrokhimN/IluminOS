use crate::framebuffer::{self, fill_rect, draw_text_at, draw_text_scaled, dimensions};
use crate::keyboard::{self, KEY_ENTER, KEY_BACKSPACE, KEY_TAB};
use crate::sound;
use alloc::string::String;

const USER: &str = "root";
const PASS: &str = "iluminos";

const BG_TOP: u32 = 0x0A0A18;      // фон сверху (тёмно-синий)
const BG_BOT: u32 = 0x1A1030;      // фон снизу (фиолетовый)
const CARD_BG: u32 = 0x15152A;     // фон карточки
const CARD_BORDER: u32 = 0x4444AA; // рамка карточки
const FIELD_BG: u32 = 0x0E0E1E;    // фон поля ввода
const FIELD_ACTIVE: u32 = 0x5566FF;// рамка активного поля
const FIELD_IDLE: u32 = 0x333355;  // рамка неактивного поля
const ACCENT: u32 = 0x66DDFF;      // акцент (голубой)
const TITLE_C: u32 = 0x88AAFF;     // заголовок
const LABEL_C: u32 = 0x8888AA;     // подписи полей
const TEXT_C: u32 = 0xDDDDEE;      // вводимый текст
const HINT_C: u32 = 0x555577;      // подсказки внизу
const ERR_C: u32 = 0xFF5566;       // ошибка

// какое поле сейчас активно
#[derive(PartialEq, Clone, Copy)]
enum Field { User, Pass }

// показать логин, вернуться только когда пароль верный
pub fn run() {
    let mut username = String::new();
    let mut password = String::new();
    let mut field = Field::User;
    let mut error = false;

    // СТАТИКУ (фон, карточку, заголовок) рисуем ОДИН раз — не мерцает
    draw_static();
    draw_dynamic(&username, &password, field, error);

    loop {
        let key = keyboard::read_key();

        match key {
            KEY_ENTER => {
                if field == Field::User {
                    field = Field::Pass;
                } else {
                    if username == USER && password == PASS {
                        success_animation();
                        return;
                    } else {
                        // ошибка: тряхнуть карточку (тут полная перерисовка,
                        // но это короткая анимация — мерцание незаметно)
                        error = true;
                        password.clear();
                        sound::beep(120, 1);
                        for i in 0..6 {
                            let shake = if i % 2 == 0 { 6 } else { -6 };
                            draw_shake(&username, &password, field, error, shake);
                            sound::delay(1);
                        }
                        // вернуть статику на место после тряски
                        draw_static();
                    }
                }
            }
            KEY_TAB => {
                field = if field == Field::User { Field::Pass } else { Field::User };
                error = false;
            }
            KEY_BACKSPACE => {
                match field {
                    Field::User => { username.pop(); }
                    Field::Pass => { password.pop(); }
                }
                error = false;
            }
            0x20..=0x7e => {
                match field {
                    Field::User => if username.len() < 20 { username.push(key as char); }
                    Field::Pass => if password.len() < 20 { password.push(key as char); }
                }
                error = false;
            }
            _ => {}
        }

        // на каждую клавишу перерисовываем ТОЛЬКО поля + ошибку (не весь экран)
        draw_dynamic(&username, &password, field, error);
    }
}

// координаты карточки (нужны и static, и dynamic — считаем одинаково)
fn card_geom() -> (usize, usize, usize, usize) {
    let (w, h) = dimensions();
    let card_w = 420usize;
    let card_h = 240usize;
    ((w - card_w) / 2, (h - card_h) / 2, card_w, card_h)
}

// фон, карточка, заголовок, подписи. Рисуется ОДИН раз
fn draw_static() {
    let (w, h) = dimensions();
    draw_gradient(w, h);

    let (card_x, card_y, card_w, card_h) = card_geom();

    // тень + тело + рамка карточки
    fill_rect(card_x + 6, card_y + 6, card_w, card_h, 0x05050A);
    fill_rect(card_x, card_y, card_w, card_h, CARD_BG);
    draw_border(card_x, card_y, card_w, card_h, 2, CARD_BORDER);

    // заголовок + подзаголовок
    let title = "IluminOS";
    let title_w = title.len() * 8 * 2;
    draw_text_scaled(title, card_x + (card_w - title_w) / 2, card_y + 24, TITLE_C, 2);
    let sub = "please sign in";
    draw_text_at(sub, card_x + (card_w - sub.len() * 8) / 2, card_y + 52, LABEL_C);

    fill_rect(card_x + 30, card_y + 74, card_w - 60, 1, CARD_BORDER);

    let fx = card_x + 40;
    draw_text_at("USERNAME", fx, card_y + 90, LABEL_C);
    draw_text_at("PASSWORD", fx, card_y + 140, LABEL_C);

    // подсказки внизу
    let hint = "Tab: switch field    Enter: confirm";
    draw_text_at(hint, (w - hint.len() * 8) / 2, h - 40, HINT_C);
    let demo = "demo: root / iluminos";
    draw_text_at(demo, (w - demo.len() * 8) / 2, h - 24, 0x666688);
}

// перерисовать ТОЛЬКО меняющееся: поля ввода + сообщение об ошибке
fn draw_dynamic(username: &str, password: &str, field: Field, error: bool) {
    let (card_x, card_y, card_w, _card_h) = card_geom();
    let fx = card_x + 40;
    let fw = card_w - 80;

    // поля (сами затирают свой фон, так что старый текст исчезает)
    draw_field(fx, card_y + 104, fw, username, false, field == Field::User);
    draw_field(fx, card_y + 154, fw, password, true, field == Field::Pass);

    // область под ошибкой: сначала затереть фоном карточки, потом (если есть) текст
    fill_rect(card_x + 20, card_y + 190, card_w - 40, 12, CARD_BG);
    if error {
        let msg = "invalid credentials";
        draw_text_at(msg, card_x + (card_w - msg.len() * 8) / 2, card_y + 194, ERR_C);
    }
}

// полная перерисовка со смещением карточки (только для анимации тряски)
fn draw_shake(username: &str, password: &str, field: Field, error: bool, shake: i32) {
    let (w, h) = dimensions();
    draw_gradient(w, h);
    let (base_x, card_y, card_w, card_h) = card_geom();
    let card_x = base_x.wrapping_add(shake as usize);

    fill_rect(card_x + 6, card_y + 6, card_w, card_h, 0x05050A);
    fill_rect(card_x, card_y, card_w, card_h, CARD_BG);
    draw_border(card_x, card_y, card_w, card_h, 2, CARD_BORDER);
    let title = "IluminOS";
    let title_w = title.len() * 8 * 2;
    draw_text_scaled(title, card_x + (card_w - title_w) / 2, card_y + 24, TITLE_C, 2);
    fill_rect(card_x + 30, card_y + 74, card_w - 60, 1, CARD_BORDER);
    let fx = card_x + 40;
    let fw = card_w - 80;
    draw_text_at("USERNAME", fx, card_y + 90, LABEL_C);
    draw_text_at("PASSWORD", fx, card_y + 140, LABEL_C);
    draw_field(fx, card_y + 104, fw, username, false, field == Field::User);
    draw_field(fx, card_y + 154, fw, password, true, field == Field::Pass);
    if error {
        let msg = "invalid credentials";
        draw_text_at(msg, card_x + (card_w - msg.len() * 8) / 2, card_y + 194, ERR_C);
    }
}

// нарисовать поле ввода: рамка + текст (или точки для пароля) + курсор
fn draw_field(x: usize, y: usize, w: usize, value: &str, secret: bool, active: bool) {
    let fh = 24usize;
    // фон поля
    fill_rect(x, y, w, fh, FIELD_BG);
    // рамка: ярче если поле активно
    let border = if active { FIELD_ACTIVE } else { FIELD_IDLE };
    draw_border(x, y, w, fh, if active { 2 } else { 1 }, border);

    // содержимое: пароль показываем точками
    let tx = x + 10;
    let ty = y + 8;
    if secret {
        // рисуем по точке на каждый символ пароля
        let mut px = tx;
        for _ in 0..value.len() {
            fill_rect(px, ty + 2, 5, 5, TEXT_C); // квадратик-«точка»
            px += 12;
        }
        // курсор после точек (если активно)
        if active {
            fill_rect(px, ty - 1, 2, 10, ACCENT);
        }
    } else {
        draw_text_at(value, tx, ty, TEXT_C);
        // курсор после текста
        if active {
            fill_rect(tx + value.len() * 8, ty - 1, 2, 10, ACCENT);
        }
    }
}

// рамка прямоугольника толщиной t
fn draw_border(x: usize, y: usize, w: usize, h: usize, t: usize, color: u32) {
    fill_rect(x, y, w, t, color);              // верх
    fill_rect(x, y + h - t, w, t, color);      // низ
    fill_rect(x, y, t, h, color);              // лево
    fill_rect(x + w - t, y, t, h, color);      // право
}

// вертикальный градиент фона от BG_TOP к BG_BOT
fn draw_gradient(w: usize, h: usize) {
    let bands = h / 4;
    // разбираем цвета на каналы для интерполяции
    let (r1, g1, b1) = ((BG_TOP >> 16) & 0xFF, (BG_TOP >> 8) & 0xFF, BG_TOP & 0xFF);
    let (r2, g2, b2) = ((BG_BOT >> 16) & 0xFF, (BG_BOT >> 8) & 0xFF, BG_BOT & 0xFF);
    for band in 0..bands {
        let t = ((band * 255) / bands.max(1)) as u32; // 0..255 по высоте (u32 для арифметики с цветами)
        let r = r1 + (r2 - r1) * t / 255;
        let g = g1 + (g2 - g1) * t / 255;
        let b = b1 + (b2 - b1) * t / 255;
        let color = (r << 16) | (g << 8) | b;
        fill_rect(0, band * 4, w, 4, color);
    }
}

// приятный аккорд-«динь» при успешном входе
fn success_animation() {
    // восходящие ноты — «успех»
    framebuffer::clear();
}
