use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};
use tokio::sync::Mutex;
use zbus::{ConnectionBuilder, fdo, interface};
use zvariant::{Value, OwnedValue};

const BUS_NAME: &str = "org.freedesktop.Notifications";
const OBJ_PATH: &str = "/org/freedesktop/Notifications";
const IFACE: &str = "org.freedesktop.Notifications";

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub replaces_id: u32,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    pub hints: HashMap<String, OwnedValue>,
    pub expire_timeout: i32,
}

#[derive(Debug, Default)]
struct State {
    next_id: AtomicU32,
    store: Mutex<HashMap<u32, Notification>>,
}

impl State {
    fn next(&self, replaces: u32) -> u32 {
        if replaces != 0 {
            replaces
        } else {
            self.next_id.fetch_add(1, Ordering::Relaxed) + 1
        }
    }
}

#[derive(Debug, Clone)]
pub struct Notifications {
    state: Arc<State>,
}

impl Notifications {
    pub fn new() -> Self {
        Self {
            state: Arc::new(State::default()),
        }
    }

    pub async fn run(self) -> zbus::Result<()> {
        ConnectionBuilder::session()?
            .name(BUS_NAME)?
            .serve_at(OBJ_PATH, Arc::new(self))?
            .build()
            .await?;

        futures_util::future::pending::<()>().await;
        Ok(())
    }

    async fn close_and_emit(
        &self,
        id: u32,
        reason: u32,
        ctxt: &zbus::Context<'_>,
    ) -> zbus::Result<()> {
        let mut map = self.state.store.lock().await;
        if map.remove(&id).is_some() {
            ctxt.signal(IFACE, "NotificationClosed", &(id, reason)).await?;
        }
        Ok(())
    }
}

#[interface(name = IFACE)]
impl Notifications {
    async fn notify(
        &self,
        _ctxt: zbus::Caller<'_>,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<&str>,
        hints: HashMap<&str, Value<'_>>,
        expire_timeout: i32,
    ) -> u32 {
        let id = self.state.next(replaces_id);
        let actions = actions.into_iter().map(|a| a.to_string()).collect();
        let hints = hints
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_owned()))
            .collect();

        let n = Notification {
            id,
            app_name: app_name.to_string(),
            replaces_id,
            app_icon: app_icon.to_string(),
            summary: summary.to_string(),
            body: body.to_string(),
            actions,
            hints,
            expire_timeout,
        };

        {
            let mut map = self.state.store.lock().await;
            map.insert(id, n.clone());
        }

        println!(
            "New notification: [{}] {} — {}",
            n.app_name, n.summary, n.body
        );

        id
    }

    async fn close_notification(&self, ctxt: zbus::Context<'_>, id: u32) -> fdo::Result<()> {
        self.close_and_emit(id, 3, &ctxt)
            .await
            .map_err(|e| fdo::Error::Failed(format!("Failed to close: {e}")))
    }

    fn get_capabilities(&self) -> Vec<&str> {
        vec!["body", "actions", "body-markup"]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "runst".into(),
            "pandgey".into(),
            env!("CARGO_PKG_VERSION").into(),
            "1.2".into(),
        )
    }
}
