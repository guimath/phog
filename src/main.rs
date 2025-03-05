use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use pgt::logic::{AppLogic, FileMoveStatus, ImageStat};

slint::include_modules!();

const MIN_DELAY: Duration = Duration::from_millis(600);

/// Syntactic sugar for async in slint callback
///
/// ### Params
/// 1) existing App Window variable
/// 2) name of ui handle to be used for ui calls inside async
/// 3) existing Mutex App Logic variable
/// 4) name of the locked logic name to be used for logic calls inside async
/// 5) all the code to be placed in async block (isolated by brackets)
macro_rules! async_context {
    ($last_date:ident, $ui:ident, $logic:ident, $code:block) => {{
        let ui_handle: slint::Weak<AppWindow> = $ui.as_weak();
        let logic_ref: Arc<Mutex<AppLogic>> = $logic.clone();
        move |repeat: bool| {
            if repeat {
                if Instant::now() - $last_date < MIN_DELAY {
                    return;
                }
                $last_date = Instant::now();
            }
            let logic_c = logic_ref.clone();
            #[allow(unused)]
            let $ui = ui_handle.unwrap();
            slint::spawn_local(async_compat::Compat::new(async move {
                #[allow(unused_mut)]
                let mut $logic = logic_c.lock().await;
                $code
            }))
            .unwrap();
        }
    }};
}

macro_rules! update_image {
    ($ui:ident, $logic:ident) => {{
        let img: ImageStat = $logic.get_img().await;
        $ui.set_photo_path(img.image);
        $ui.set_photo_num(img.number as i32);
        $ui.set_total_num(img.out_of as i32);
        $ui.set_photo_name(img.name.into());
    }};
}

// no #[tokio::main] because it crashes after a few Mutex locks (compatibility issue with slint)
fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    ui.window().set_maximized(true);
    // TODO PARAM
    // let folder_path = PathBuf::from_str("/home/guilhem/Pictures/TEST").unwrap();
    let folder_path = PathBuf::from_str("/home/guilhem/Documents/SAVE/Photos/2024_20 - Palawan").unwrap();
    let logic = Arc::new(Mutex::new(AppLogic::new(folder_path)));
    let first: ImageStat = logic.blocking_lock().get_first_img();
    ui.set_total_num(first.out_of as i32);
    ui.set_photo_num(first.number as i32);
    ui.set_photo_name(first.name.into());
    ui.set_photo_path(first.image);
    let mut last_cmd = Instant::now();

    let logic_c = logic.clone();
    slint::spawn_local(async_compat::Compat::new(async move {
        let mut logic = logic_c.lock().await;
        logic.init().await;
    }))
    .unwrap();

    ui.on_next(async_context! {last_cmd, ui, logic, {
        if logic.next_img().await{
            update_image!(ui, logic);
        }
    }});

    ui.on_prev(async_context! {last_cmd, ui, logic, {
        if logic.prev_img().await {
            update_image!(ui, logic);
        }
    }});
    ui.on_edit(async_context! {last_cmd, ui, logic, {
        match logic.edit(){
            FileMoveStatus::Successfull => ui.set_message("Copied to edit successfully".into()),
            FileMoveStatus::NoRAW => ui.set_message("Copied JPG to edit, no Raw found ".into()),
            FileMoveStatus::Failed => ui.set_message("Copy unsuccessful".into()),
            FileMoveStatus::AlreadyDone => ui.set_message("Already copied".into()),
        }
        ui.set_show_message(true);
        ui.set_message_sec_up(1.3);
    }});

    ui.on_delete(async_context! {last_cmd, ui, logic, {
        let (status, to_update) = logic.delete().await;
        match status{
            FileMoveStatus::Successfull => ui.set_message("Moved to bin successfully".into()),
            FileMoveStatus::NoRAW => ui.set_message("Moved JPG to edit, no Raw found ".into()),
            FileMoveStatus::Failed => ui.set_message("Move to bin failed".into()),
            FileMoveStatus::AlreadyDone => ui.set_message("Already deleted".into()),
        }
        ui.set_show_message(true);
        ui.set_message_sec_up(1.2);
        if to_update {
            update_image!(ui, logic);
        }
        else {
            slint::quit_event_loop().unwrap();
        }
    }});

    ui.on_close(|| {
        slint::quit_event_loop().unwrap();
    });

    ui.run()?;

    Ok(())
}
