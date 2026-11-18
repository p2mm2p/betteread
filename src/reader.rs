use crate::page::{self, HighlightHandle, PageSentence};
use crate::settings;
use brcore::{Motion, Navigation, ReadingSettings, highlight_css};
use js_sys::{Array, Function, Reflect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    Document, Element, Event, HtmlStyleElement, KeyboardEvent, MutationObserver,
    MutationObserverInit, Window,
};

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    static REBUILD_TIMEOUT: Cell<Option<i32>> = const { Cell::new(None) };
}

/// 开启或停止当前页面的阅读模式。
///
/// 返回操作后的活动状态。页面没有可读文本或初始化失败时返回 `false`。
pub async fn toggle() -> bool {
    if SESSION.with(|slot| slot.borrow().is_some()) {
        stop();
        return false;
    }

    let loaded = settings::load().await;
    match Session::start(loaded) {
        Ok(Some(session)) => {
            SESSION.with(|slot| {
                *slot.borrow_mut() = Some(session);
            });
            true
        }
        Ok(None) => false,
        Err(error) => {
            oxichrome::log!("could not start reading mode: {error}");
            false
        }
    }
}

fn stop() {
    let session = SESSION.with(|slot| slot.borrow_mut().take());
    if let Some(session) = session {
        session.stop();
    }
}

struct Session {
    state: Rc<RefCell<State>>,
    key_listener: Closure<dyn FnMut(KeyboardEvent)>,
    resize_listener: Closure<dyn FnMut(Event)>,
    storage_event: JsValue,
    storage_listener: Closure<dyn FnMut(JsValue, JsValue)>,
    mutation_observer: MutationObserver,
    // 保持回调存活：MutationObserver 只保存 JS 函数引用。
    #[allow(dead_code)]
    mutation_listener: Closure<dyn FnMut(Array, MutationObserver)>,
}

struct State {
    window: Window,
    document: Document,
    style: HtmlStyleElement,
    highlight: HighlightHandle,
    sentences: Vec<PageSentence>,
    navigation: Navigation,
}

type StorageListener = (JsValue, Closure<dyn FnMut(JsValue, JsValue)>);

impl Session {
    fn start(loaded: ReadingSettings) -> Result<Option<Self>, String> {
        let window = web_sys::window().ok_or_else(|| "no browser window".to_string())?;
        let document = window
            .document()
            .ok_or_else(|| "no page document".to_string())?;
        let Some(snapshot) = page::scan(&document)? else {
            return Ok(None);
        };
        let Some(navigation) = Navigation::new(snapshot.blocks) else {
            return Ok(None);
        };
        let style = page::install_style(&document, &loaded)?;
        let highlight = match HighlightHandle::new(&window) {
            Ok(highlight) => highlight,
            Err(error) => {
                style.remove();
                return Err(error);
            }
        };
        let active_range = snapshot.sentences[navigation.active_sentence()]
            .range
            .clone();
        if let Err(error) = highlight.show(&active_range) {
            style.remove();
            return Err(error);
        }
        page::ensure_visible(&window, &active_range);

        let state = Rc::new(RefCell::new(State {
            window: window.clone(),
            document: document.clone(),
            style,
            highlight,
            sentences: snapshot.sentences,
            navigation,
        }));

        let key_listener = {
            let state = Rc::clone(&state);
            Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
                handle_key(&state, &event);
            })
        };
        document
            .add_event_listener_with_callback("keydown", key_listener.as_ref().unchecked_ref())
            .map_err(js_error)?;

        let resize_listener = {
            let state = Rc::clone(&state);
            Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
                schedule_rebuild(&state);
            })
        };
        window
            .add_event_listener_with_callback("resize", resize_listener.as_ref().unchecked_ref())
            .map_err(js_error)?;

        let (storage_event, storage_listener) = install_storage_listener(&state)?;

        let mutation_listener = {
            let state = Rc::clone(&state);
            Closure::<dyn FnMut(Array, MutationObserver)>::new(
                move |_records: Array, _observer: MutationObserver| {
                    schedule_rebuild(&state);
                },
            )
        };
        let mutation_observer =
            MutationObserver::new(mutation_listener.as_ref().unchecked_ref()).map_err(js_error)?;
        let observer_options = MutationObserverInit::new();
        observer_options.set_child_list(true);
        observer_options.set_character_data(true);
        observer_options.set_subtree(true);
        let observed_root = document
            .document_element()
            .ok_or_else(|| "page has no document element".to_string())?;
        mutation_observer
            .observe_with_options(observed_root.as_ref(), &observer_options)
            .map_err(js_error)?;

        Ok(Some(Self {
            state,
            key_listener,
            resize_listener,
            storage_event,
            storage_listener,
            mutation_observer,
            mutation_listener,
        }))
    }

    fn stop(self) {
        let state = self.state.borrow();
        let _ = state.document.remove_event_listener_with_callback(
            "keydown",
            self.key_listener.as_ref().unchecked_ref(),
        );
        let _ = state.window.remove_event_listener_with_callback(
            "resize",
            self.resize_listener.as_ref().unchecked_ref(),
        );
        remove_storage_listener(
            &self.storage_event,
            self.storage_listener.as_ref().unchecked_ref(),
        );
        self.mutation_observer.disconnect();
        state.highlight.clear();
        state.style.remove();
        clear_rebuild_timeout(&state.window);
    }
}

fn install_storage_listener(state: &Rc<RefCell<State>>) -> Result<StorageListener, String> {
    let chrome = Reflect::get(&js_sys::global(), &JsValue::from_str("chrome")).map_err(js_error)?;
    if chrome.is_undefined() || chrome.is_null() {
        return Err("chrome.storage is unavailable in this page".to_string());
    }
    let storage = Reflect::get(&chrome, &JsValue::from_str("storage")).map_err(js_error)?;
    let on_changed = Reflect::get(&storage, &JsValue::from_str("onChanged")).map_err(js_error)?;
    let add_listener = Reflect::get(&on_changed, &JsValue::from_str("addListener"))
        .map_err(js_error)?
        .dyn_into::<Function>()
        .map_err(|_| "chrome.storage.onChanged.addListener is unavailable".to_string())?;

    let state = Rc::clone(state);
    let listener = Closure::<dyn FnMut(JsValue, JsValue)>::new(
        move |_changes: JsValue, area_name: JsValue| {
            if area_name.as_string().as_deref() != Some("local") {
                return;
            }
            let state = Rc::clone(&state);
            wasm_bindgen_futures::spawn_local(async move {
                let loaded = settings::load().await;
                apply_settings(&state, loaded);
            });
        },
    );
    add_listener
        .call1(&on_changed, listener.as_ref().unchecked_ref())
        .map_err(js_error)?;
    Ok((on_changed, listener))
}

fn remove_storage_listener(event: &JsValue, listener: &Function) {
    if let Ok(remove_listener) = Reflect::get(event, &JsValue::from_str("removeListener"))
        .and_then(|value| value.dyn_into::<Function>())
    {
        let _ = remove_listener.call1(event, listener);
    }
}

fn apply_settings(state: &Rc<RefCell<State>>, loaded: ReadingSettings) {
    let state = state.borrow();
    state.style.set_text_content(Some(&highlight_css(&loaded)));
}

fn handle_key(state: &Rc<RefCell<State>>, event: &KeyboardEvent) {
    if event.default_prevented()
        || event.is_composing()
        || event.key_code() == 229
        || event.alt_key()
        || event.ctrl_key()
        || event.meta_key()
    {
        return;
    }
    if is_editable_target(event.target()) {
        return;
    }
    let motion = match event.key().as_str() {
        "j" => Motion::NextBlock,
        "k" => Motion::PreviousBlock,
        "h" => Motion::PreviousSentence,
        "l" => Motion::NextSentence,
        _ => return,
    };

    let changed = state.borrow_mut().navigation.apply(motion);
    if changed && refresh_active(state, true).is_ok() {
        event.prevent_default();
    }
}

fn is_editable_target(target: Option<web_sys::EventTarget>) -> bool {
    let Some(target) = target else {
        return false;
    };
    let Ok(element) = target.dyn_into::<Element>() else {
        return false;
    };
    let tag = element.tag_name().to_ascii_lowercase();
    if matches!(
        tag.as_str(),
        "button" | "input" | "option" | "select" | "textarea"
    ) {
        return true;
    }
    element
        .closest("[contenteditable]")
        .ok()
        .flatten()
        .is_some_and(|editable| {
            editable.get_attribute("contenteditable").as_deref() != Some("false")
        })
}

fn refresh_active(state: &Rc<RefCell<State>>, scroll: bool) -> Result<(), String> {
    let state = state.borrow();
    let Some(sentence) = state.sentences.get(state.navigation.active_sentence()) else {
        return Ok(());
    };
    state.highlight.show(&sentence.range)?;
    if scroll {
        page::ensure_visible(&state.window, &sentence.range);
    }
    Ok(())
}

fn schedule_rebuild(state: &Rc<RefCell<State>>) {
    let window = state.borrow().window.clone();
    clear_rebuild_timeout(&window);
    let state = Rc::clone(state);
    let callback: Function = Closure::once_into_js(move || rebuild(&state)).unchecked_into();
    match window.set_timeout_with_callback_and_timeout_and_arguments_0(&callback, 160) {
        Ok(timeout) => REBUILD_TIMEOUT.with(|slot| slot.set(Some(timeout))),
        Err(error) => {
            oxichrome::log!("could not schedule layout refresh: {}", js_error(error));
        }
    }
}

fn clear_rebuild_timeout(window: &Window) {
    if let Some(timeout) = REBUILD_TIMEOUT.with(|slot| slot.replace(None)) {
        window.clear_timeout_with_handle(timeout);
    }
}

fn rebuild(state: &Rc<RefCell<State>>) {
    REBUILD_TIMEOUT.with(|slot| slot.set(None));
    let (document, old_index, old_text) = {
        let state = state.borrow();
        let old_index = state.navigation.active_sentence();
        let old_text = state
            .sentences
            .get(old_index)
            .map(|sentence| sentence.text.clone());
        (state.document.clone(), old_index, old_text)
    };

    let Some(snapshot) = page::scan(&document).ok().flatten() else {
        state.borrow().highlight.clear();
        return;
    };
    let sentence_count = snapshot.sentences.len();
    let new_index = old_text
        .and_then(|old_text| {
            let start = old_index.saturating_sub(20);
            let end = (old_index + 20).min(sentence_count);
            snapshot
                .sentences
                .iter()
                .enumerate()
                .skip(start)
                .take(end.saturating_sub(start))
                .find(|(_, sentence)| sentence.text == old_text)
                .map(|(index, _)| index)
        })
        .unwrap_or_else(|| old_index.min(sentence_count.saturating_sub(1)));

    let Some(mut navigation) = Navigation::new(snapshot.blocks) else {
        state.borrow().highlight.clear();
        return;
    };
    navigation.set_active(new_index);
    let active_range = snapshot.sentences[new_index].range.clone();

    let mut state = state.borrow_mut();
    state.sentences = snapshot.sentences;
    state.navigation = navigation;
    let _ = state.highlight.show(&active_range);
    page::ensure_visible(&state.window, &active_range);
}

fn js_error(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}
