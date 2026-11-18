use brcore::{HighlightColor, HighlightStyle, ReadingSettings};
use leptos::prelude::*;
use leptos::task::spawn_local;
use oxichrome::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Status {
    Saved,
    Error(String),
}

#[options_page(style = ["base.css", "style.css"])]
fn Options() -> impl IntoView {
    let current = RwSignal::new(ReadingSettings::default());
    let color_input = RwSignal::new(ReadingSettings::default().color.to_hex());
    let status = RwSignal::new(None::<Status>);

    spawn_local(async move {
        let loaded = crate::settings::load().await;
        color_input.set(loaded.color.to_hex());
        current.set(loaded);
    });

    let update_style = move |event| {
        let value = event_target_value(&event);
        if let Ok(style) = value.parse::<HighlightStyle>() {
            current.update(|settings| settings.style = style);
            status.set(None);
        } else {
            status.set(Some(Status::Error(
                "样式无效。请选择下划线或块。".to_string(),
            )));
        }
    };
    let update_color = move |event| {
        color_input.set(event_target_value(&event));
        status.set(None);
    };
    let save = move |_| {
        let candidate_color = color_input.get_untracked();
        let color = match candidate_color.parse::<HighlightColor>() {
            Ok(color) => color,
            Err(_) => {
                status.set(Some(Status::Error(
                    "颜色格式无效。请输入 #RRGGBB，例如 #ffd54f。".to_string(),
                )));
                return;
            }
        };
        let mut candidate = current.get_untracked();
        candidate.color = color;
        status.set(None);
        spawn_local(async move {
            match crate::settings::save(&candidate).await {
                Ok(()) => {
                    current.set(candidate);
                    status.set(Some(Status::Saved));
                }
                Err(error) => {
                    oxichrome::log!("保存设置失败：{error}");
                    status.set(Some(Status::Error(
                        "保存失败。请重新加载扩展后重试。".to_string(),
                    )));
                }
            }
        });
    };

    view! {
        <main class="page">
            <header class="head">
                <img class="mark" src="/icons/128.png" alt="" />
                <div class="titles">
                    <h1>"BettRead"</h1>
                    <p>"逐句阅读，使用 Vim 风格按键移动。"</p>
                </div>
            </header>

            <section class="card">
                <div class="setting">
                    <div class="main">
                        <div class="label">"高亮样式"</div>
                        <div class="hint">"下划线在文字下方画线，块样式填充文字背景。"</div>
                    </div>
                    <select
                        class="select"
                        on:change=update_style
                        prop:value=move || current.with(|settings| settings.style.as_str())
                    >
                        <option value="underline">"下划线"</option>
                        <option value="block">"块"</option>
                    </select>
                </div>
                <div class="setting">
                    <div class="main">
                        <div class="label">"高亮颜色"</div>
                        <div class="hint">"颜色格式为 #RRGGBB。"</div>
                    </div>
                    <input
                        class="color-input mono"
                        type="text"
                        spellcheck="false"
                        placeholder="#ffd54f"
                        prop:value=move || color_input.get()
                        on:input=update_color
                    />
                </div>
            </section>

            <section class="card pad stack">
                <button class="btn primary" on:click=save>"保存设置"</button>
                <p
                    class="notice"
                    class:ok=move || matches!(status.get(), Some(Status::Saved))
                    class:danger=move || matches!(status.get(), Some(Status::Error(_)))
                >
                    {move || match status.get() {
                        Some(Status::Saved) => "设置已保存。".to_string(),
                        Some(Status::Error(error)) => error,
                        None => String::new(),
                    }}
                </p>
            </section>

            <section class="card pad stack">
                <div class="label">"阅读操作"</div>
                <p class="hint">"开启或停止阅读模式：按 Alt+R，或点击工具栏图标。"</p>
                <p class="hint">"移动到下一段：按 j。"</p>
                <p class="hint">"移动到上一段：按 k。"</p>
                <p class="hint">"移动句子：按 h 和 l。"</p>
            </section>
        </main>
    }
}
