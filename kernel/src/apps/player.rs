use crate::keyboard;
use crate::sound::{self, SongPlayer};
use crate::print_color;
use crate::framebuffer::{GREEN, RED, CYAN, GRAY};

include!(concat!(env!("OUT_DIR"), "/songs_generated.rs"));

pub const PLAYER_SAMPLE_RATE: u16 = 11025;

pub fn find(name: &str) -> Option<&'static [u8]> {
    SONGS.iter().find(|(n, _)| *n == name).map(|(_, data)| *data)
}

pub fn list() {
    if SONGS.is_empty() {
        print_color!(GRAY, "  (пусто -- положите .raw файлы в src/apps/music/ и пересоберите ядро)\n");
        return;
    }
    for (name, data) in SONGS.iter() {
        let secs = data.len() as u32 / PLAYER_SAMPLE_RATE as u32;
        print_color!(CYAN, "  {}", name);
        print_color!(GRAY, "  (~{}s, {} bytes)\n", secs, data.len());
    }
}

pub fn play(name: &str) -> bool {
    let Some(data) = find(name) else {
        return false;
    };
    if data.is_empty() {
        return false;
    }

    let mut player = SongPlayer::new(data);

    if !sound::stream_start(&mut player, PLAYER_SAMPLE_RATE) {
        print_color!(RED, "  не удалось выделить буфер под плеер (нет памяти?)\n");
        return false;
    }

    print_color!(GREEN, "  playing \"{}\"... esc to stop\n", name);

    loop {
        sound::stream_tick(&mut player);

        if let Some((key, true)) = keyboard::try_read_key_event() {
            if key == 0x1b {
                break; 
            }
        }

        if player.finished() && player.drained() {
            break; 
        }

        core::hint::spin_loop();
    }

    sound::stop();
    true
}
