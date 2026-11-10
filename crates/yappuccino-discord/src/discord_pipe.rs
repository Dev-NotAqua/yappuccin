//! Windows Discord IPC pipe, for games such as FiveM that ask the local
//! Discord client who is signed in. Protocol:
//! https://docs.discord.food/topics/rpc#ipc-transport.
//! Only who is signed in, Rich Presence's acknowledgements and FiveM's
//! authorization are exposed; everything else is refused.

use std::{io, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
    sync::watch,
};
use url::Url;

use crate::{
    api::Api,
    oauth,
    service::{Account, Snapshot, Status},
};

/// FiveM's Discord application, the only application this pipe authorizes.
const FIVEM: &str = "1473308775258525930";
const MAX_PACKET: usize = 64 * 1024;
const WAIT: Duration = Duration::from_secs(15);
/// Discord's clients try these slots in order, so a running Discord keeps
/// the lower ones and games find it first.
const SLOTS: std::ops::Range<u8> = 0..10;

pub(crate) fn start(api: Arc<Api>, mut snapshots: watch::Receiver<Arc<Snapshot>>) {
    // Tests must opt out to avoid answering their fixture account on the
    // desktop. An explicit variable also helps when another pipe is wanted.
    let disabled = std::env::var_os("YAPPUCCINO_DISCORD_IPC").is_some_and(|v| v == "0")
        || crate::client::test_server().is_some();
    if disabled {
        return;
    }
    tokio::spawn(async move {
        // Binding only while signed in keeps an offline pipe from answering
        // games with an account that can't be checked yet.
        if online(&mut snapshots).await.is_none() {
            return;
        }
        let (server, path) = match bind() {
            Ok(pipe) => pipe,
            Err(error) => {
                tracing::warn!(%error, "Discord IPC pipe unavailable");
                return;
            }
        };
        tracing::info!(pipe = %path, "Discord IPC pipe listening");
        serve(server, path, api, snapshots).await;
    });
}

/// The first free Discord IPC slot, keeping the lower ones a running Discord
/// client already owns, so games find that client first.
fn bind() -> Result<(NamedPipeServer, String)> {
    for slot in SLOTS {
        let path = format!(r"\\.\pipe\discord-ipc-{slot}");
        match ServerOptions::new().first_pipe_instance(true).create(&path) {
            Ok(server) => return Ok((server, path)),
            // Discord, or another copy of Yappuccino, has this slot.
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {}
            Err(error) => return Err(error.into()),
        }
    }
    bail!("no free Discord IPC pipe")
}

async fn serve(
    mut server: NamedPipeServer,
    path: String,
    api: Arc<Api>,
    snapshots: watch::Receiver<Arc<Snapshot>>,
) {
    loop {
        if let Err(error) = server.connect().await {
            tracing::warn!(%error, "Discord IPC pipe stopped");
            return;
        }
        let connected = server;
        // The next instance must exist before this one is handed to its task,
        // or a client connecting in between finds no pipe.
        match ServerOptions::new().create(&path) {
            Ok(next) => server = next,
            Err(error) => {
                tracing::warn!(%error, "Discord IPC pipe stopped accepting clients");
                tokio::spawn(connection(connected, api, snapshots));
                return;
            }
        }
        tokio::spawn(connection(connected, api.clone(), snapshots.clone()));
    }
}

async fn connection(
    mut pipe: NamedPipeServer,
    api: Arc<Api>,
    mut snapshots: watch::Receiver<Arc<Snapshot>>,
) -> Result<()> {
    let (opcode, payload) = tokio::time::timeout(WAIT, read(&mut pipe)).await??;
    let handshake: Value = serde_json::from_slice(&payload)?;
    let client = handshake["client_id"].as_str().unwrap_or_default();
    if opcode != 0 || handshake["v"] != 1 || !snowflake(client) {
        close(&mut pipe, 4000, "Invalid IPC handshake").await?;
        return Ok(());
    }
    let client = client.to_owned();
    // A connected, idle game must not retain its handshake JSON/packet buffer.
    drop(handshake);
    drop(payload);
    let Some((user, account)) = online(&mut snapshots).await else {
        return Ok(());
    };
    send(&mut pipe, ready(&user, account.as_ref())).await?;
    loop {
        let (opcode, payload) = read(&mut pipe).await?;
        match opcode {
            2 => return Ok(()),
            3 => {
                write(&mut pipe, 4, &payload).await?;
                continue;
            }
            4 => continue,
            1 => {}
            _ => bail!("invalid IPC opcode"),
        }
        let packet: Value = serde_json::from_slice(&payload)?;
        drop(payload);
        let cmd = packet["cmd"].as_str().unwrap_or_default().to_owned();
        let nonce = packet["nonce"].clone();
        let result: Result<Value> = match cmd.as_str() {
            "AUTHORIZE" => authorize(&api, &client, &packet["args"])
                .await
                .map(|code| json!({"code": code})),
            // This account's Rich Presence is Yappuccino's own; acknowledge
            // games that share the pipe without showing a second one.
            "SET_ACTIVITY" | "SUBSCRIBE" | "UNSUBSCRIBE" => Ok(Value::Null),
            "GET_CURRENT_USER" => Ok(user_json(&user, account.as_ref())),
            _ => Err(anyhow::anyhow!("Unsupported IPC command")),
        };
        match result {
            Ok(data) => {
                send(
                    &mut pipe,
                    json!({"cmd": cmd, "nonce": nonce, "evt": Value::Null, "data": data}),
                )
                .await?
            }
            Err(error) => {
                tracing::debug!(%error, command = %cmd, "Discord IPC command refused");
                send(
                    &mut pipe,
                    json!({"cmd": cmd, "nonce": nonce, "evt": "ERROR",
                    "data": {"code": 4000, "message": "Unsupported IPC command"}}),
                )
                .await?;
            }
        }
    }
}

/// Waits for the account to be connected, and gives who it is.
async fn online(
    snapshots: &mut watch::Receiver<Arc<Snapshot>>,
) -> Option<(String, Option<Account>)> {
    loop {
        {
            let snapshot = snapshots.borrow_and_update();
            if snapshot.status == Status::Online
                && let Some(user) = snapshot.user_id.as_deref().filter(|id| snowflake(id))
            {
                return Some((user.to_owned(), snapshot.account.clone()));
            }
        }
        snapshots.changed().await.ok()?;
    }
}

/// Authorizes FiveM's application for this account, and gives the code
/// Cfx's server exchanges for the guilds it checks.
async fn authorize(api: &Api, client: &str, args: &Value) -> Result<String> {
    // A local application must not be able to authorize anything else, or
    // to choose scopes this pipe didn't agree to.
    ensure!(client == FIVEM, "Only FiveM's application is supported");
    ensure!(
        args["client_id"].as_str() == Some(FIVEM),
        "Only FiveM's application is supported"
    );
    let answer = tokio::time::timeout(
        WAIT,
        api.authorize_oauth(&fivem_request().authorize_query(), false, None, None),
    )
    .await??;
    // Discord's client reads `location`; docs.discord.food names it `url`.
    let location = answer["location"]
        .as_str()
        .or_else(|| answer["url"].as_str())
        .context("Discord authorized FiveM without a redirect")?;
    code(location).context("Discord's redirect had no code")
}

/// What FiveM's authorization asks for, as Cfx's server expects it: the
/// scopes it checks, and its registered redirect.
fn fivem_request() -> oauth::Request {
    oauth::Request {
        client: FIVEM.to_owned(),
        scopes: vec!["identify".to_owned(), "guilds.join".to_owned()],
        response_type: Some("code".to_owned()),
        redirect_uri: Some("https://cfx.re".to_owned()),
        code_challenge: None,
        code_challenge_method: None,
        state: None,
        nonce: None,
        permissions: None,
        guild: None,
        guild_locked: false,
        user_install: false,
    }
}

/// The code in where Discord says the authorization goes next.
fn code(location: &str) -> Option<String> {
    Url::parse(location)
        .ok()?
        .query_pairs()
        .find(|(name, _)| name == "code")
        .map(|(_, value)| value.into_owned())
        .filter(|value| !value.is_empty())
}

fn ready(user: &str, account: Option<&Account>) -> Value {
    json!({"cmd":"DISPATCH", "evt":"READY", "nonce":Value::Null,
        "data":{"v":1,"config":{"cdn_host":"cdn.discordapp.com","api_endpoint":"//discord.com/api","environment":"production"},
        "user":user_json(user, account)}})
}

fn user_json(id: &str, account: Option<&Account>) -> Value {
    let username = account
        .map(|account| {
            if account.username.is_empty() {
                account.name.as_str()
            } else {
                account.username.as_str()
            }
        })
        .unwrap_or("yappuccino");
    json!({"id":id, "username":username,
        "global_name":account.map(|account| account.name.as_str()).filter(|name| !name.is_empty()),
        "discriminator":"0", "avatar":Value::Null, "bot":false})
}

fn snowflake(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 20
        && value.bytes().all(|b| b.is_ascii_digit())
        && value.parse::<u64>().is_ok_and(|id| id > 0)
}

async fn read<S: AsyncRead + Unpin>(pipe: &mut S) -> Result<(u32, Vec<u8>)> {
    let opcode = pipe.read_u32_le().await?;
    let size = pipe.read_u32_le().await? as usize;
    ensure!(size <= MAX_PACKET, "IPC packet too large");
    let mut payload = vec![0; size];
    pipe.read_exact(&mut payload).await?;
    Ok((opcode, payload))
}

async fn write<S: AsyncWrite + Unpin>(pipe: &mut S, opcode: u32, payload: &[u8]) -> Result<()> {
    tokio::time::timeout(WAIT, async {
        pipe.write_u32_le(opcode).await?;
        pipe.write_u32_le(payload.len() as u32).await?;
        pipe.write_all(payload).await
    })
    .await??;
    Ok(())
}

async fn send<S: AsyncWrite + Unpin>(pipe: &mut S, value: Value) -> Result<()> {
    write(pipe, 1, &serde_json::to_vec(&value)?).await
}

async fn close<S: AsyncWrite + Unpin>(pipe: &mut S, code: u32, message: &str) -> Result<()> {
    write(
        pipe,
        2,
        &serde_json::to_vec(&json!({"code":code, "message":message}))?,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> Account {
        Account {
            name: "Display Name".into(),
            username: "handle".into(),
            avatar: "https://cdn.discordapp.com/avatars/1/avatar.png".into(),
            decoration: None,
            nameplate: None,
            premium_type: 0,
            mfa_enabled: false,
            has_email: false,
        }
    }

    #[test]
    fn ready_names_the_signed_in_account_for_fivem() {
        let value = ready("123456789012345678", Some(&account()));
        assert_eq!(value["evt"], "READY");
        assert_eq!(value["data"]["v"], 1);
        assert_eq!(value["data"]["user"]["id"], "123456789012345678");
        assert_eq!(value["data"]["user"]["username"], "handle");
        assert_eq!(value["data"]["user"]["global_name"], "Display Name");
        assert_eq!(value["data"]["user"]["discriminator"], "0");
        assert_eq!(value["data"]["user"]["bot"], false);
    }

    #[test]
    fn user_falls_back_to_the_display_name_and_then_to_yappuccino() {
        let mut account = account();
        account.username.clear();
        assert_eq!(user_json("42", Some(&account))["username"], "Display Name");
        account.name.clear();
        assert!(user_json("42", Some(&account))["global_name"].is_null());
        assert_eq!(user_json("42", None)["username"], "yappuccino");
    }

    #[test]
    fn fivem_authorization_asks_only_for_what_cfx_checks() {
        assert_eq!(
            fivem_request().authorize_query(),
            "client_id=1473308775258525930&response_type=code&redirect_uri=https%3A%2F%2Fcfx.re&scope=identify+guilds.join"
        );
    }

    #[test]
    fn reads_the_code_from_where_discord_redirects() {
        assert_eq!(
            code("https://cfx.re/?state=abc&code=one-time-code").as_deref(),
            Some("one-time-code")
        );
        assert_eq!(code("https://cfx.re/?state=abc"), None);
        assert_eq!(code("https://cfx.re/?code="), None);
        assert_eq!(code("not an address"), None);
    }

    #[test]
    fn only_snowflakes_can_name_an_account() {
        assert!(snowflake("1473308775258525930"));
        assert!(!snowflake("0"));
        assert!(!snowflake(""));
        assert!(!snowflake("not-a-snowflake"));
        assert!(!snowflake("123456789012345678901"));
    }

    #[tokio::test]
    async fn refuses_to_authorize_another_application() {
        let api = Api::new("test-token".into()).unwrap();
        assert!(
            authorize(&api, "42", &json!({"client_id": FIVEM}))
                .await
                .is_err()
        );
        assert!(
            authorize(&api, FIVEM, &json!({"client_id": "42"}))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn answers_fivems_handshake_over_a_named_pipe() {
        use std::sync::atomic::{AtomicU64, Ordering};
        use tokio::net::windows::named_pipe::ClientOptions;

        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = format!(
            r"\\.\pipe\yappuccino-ipc-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&path)
            .unwrap();
        let snapshot = Snapshot {
            status: Status::Online,
            user_id: Some("123456789012345678".into()),
            account: Some(account()),
            ..Snapshot::default()
        };
        let (_snapshots, receiver) = watch::channel(Arc::new(snapshot));
        let api = Arc::new(Api::new("test-token".into()).unwrap());
        let service = tokio::spawn(serve(server, path.clone(), api, receiver));
        let mut client = tokio::time::timeout(WAIT, async {
            loop {
                match ClientOptions::new().open(&path) {
                    Ok(client) => return client,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        tokio::task::yield_now().await;
                    }
                    Err(error) => panic!("{error}"),
                }
            }
        })
        .await
        .unwrap();
        let handshake = serde_json::to_vec(&json!({"v":1,"client_id":FIVEM})).unwrap();
        write(&mut client, 0, &handshake).await.unwrap();
        let (opcode, payload) = read(&mut client).await.unwrap();
        let ready: Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(opcode, 1);
        assert_eq!(ready["evt"], "READY");
        assert_eq!(ready["data"]["user"]["id"], "123456789012345678");
        write(&mut client, 3, br#"{"ping":1}"#).await.unwrap();
        assert_eq!(
            read(&mut client).await.unwrap(),
            (4, br#"{"ping":1}"#.to_vec())
        );
        service.abort();
    }
}
