use oxichrome::core::event::FromEventArg;
use oxichrome::prelude::*;

#[background]
async fn start() {
    oxichrome::log!("BettRead background is ready");
}

#[on(runtime::on_installed)]
async fn installed() {
    oxichrome::log!("BettRead was installed or updated");
}

#[command(description = "Toggle BettRead reading mode", shortcut = "Alt+R")]
async fn toggle_reading_command(press: commands::Press) {
    toggle_tab(press.tab_id).await;
}

#[on(action::on_clicked)]
async fn toolbar_clicked(tab: wasm_bindgen::JsValue) {
    let tab_id = tabs::Tab::from_event_arg(tab).ok().and_then(|tab| tab.id);
    toggle_tab(tab_id).await;
}

async fn toggle_tab(tab_id: Option<i32>) {
    let Some(tab_id) = tab_id else {
        return;
    };
    match crate::content_script::toggle_reading::SCRIPT
        .run(tab_id, ())
        .await
    {
        Ok(state) => {
            oxichrome::log!("reading mode active: {}", state.active);
        }
        Err(error) => {
            oxichrome::log!("could not toggle reading mode: {error}");
        }
    }
}
