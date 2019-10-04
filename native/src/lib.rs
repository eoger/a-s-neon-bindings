use ffi_support::{ConcurrentHandleMap, HandleError};
use fxa_client::{scopes, FirefoxAccount};
use logins::PasswordEngine;
use neon::prelude::*;
use neon::register_module;
use std::sync::{Arc, Mutex};

static CONTENT_URL: &str = "https://accounts.firefox.com";
static CLIENT_ID: &str = "e7ce535d93522896";
static REDIRECT_URI: &str = "https://lockbox.firefox.com/fxa/android-redirect.html";
static TOKENSERVER_URL: &str = "https://token.services.mozilla.com/";

static LOGINS_KEY: &str = "deadbeef";

lazy_static::lazy_static! {
    static ref ACCOUNTS: ConcurrentHandleMap<FirefoxAccount> = ConcurrentHandleMap::new();
    static ref PASSWORDS: ConcurrentHandleMap<Arc<Mutex<PasswordEngine>>> = ConcurrentHandleMap::new();
}

fn fxa_new(mut cx: FunctionContext) -> JsResult<JsString> {
    let handle = ACCOUNTS.insert(FirefoxAccount::new(CONTENT_URL, CLIENT_ID, REDIRECT_URI));
    // We return the handle as String because u64 -> f64 doesn't work well somehow, probably conversion bs.
    Ok(cx.string(handle.into_u64().to_string()))
}

fn fxa_begin_oauth_flow(mut cx: FunctionContext) -> JsResult<JsString> {
    let handle = cx.argument::<JsString>(0)?.value().parse::<u64>().unwrap();
    let url = ACCOUNTS
        .get_mut_u64(handle, |fxa| -> Result<_, HandleError> {
            Ok(fxa
                .begin_oauth_flow(&[scopes::OLD_SYNC, scopes::PROFILE])
                .unwrap())
        })
        .unwrap();
    Ok(cx.string(url))
}

fn fxa_complete_oauth_flow(mut cx: FunctionContext) -> JsResult<JsBoolean> {
    let handle = cx.argument::<JsString>(0)?.value().parse::<u64>().unwrap();
    let code = cx.argument::<JsString>(1)?.value();
    let state = cx.argument::<JsString>(2)?.value();
    let success = ACCOUNTS
        .get_mut_u64(handle, |fxa| -> Result<_, HandleError> {
            Ok(fxa.complete_oauth_flow(&code, &state).is_ok())
        })
        .unwrap();
    Ok(cx.boolean(success))
}

fn fxa_get_access_token(mut cx: FunctionContext) -> JsResult<JsString> {
    let handle = cx.argument::<JsString>(0)?.value().parse::<u64>().unwrap();
    let json = ACCOUNTS
        .get_mut_u64(handle, |fxa| -> Result<_, HandleError> {
            let access_token = fxa
                .get_access_token(&format!("{} {}", scopes::OLD_SYNC, scopes::PROFILE))
                .unwrap();
            let json = serde_json::to_string(&access_token).unwrap();
            Ok(json)
        })
        .unwrap();
    Ok(cx.string(json))
}

fn logins_new(mut cx: FunctionContext) -> JsResult<JsString> {
    let db_path = cx.argument::<JsString>(0)?.value();
    let handle = PASSWORDS.insert(Arc::new(Mutex::new(
        PasswordEngine::new(db_path, Some(LOGINS_KEY)).unwrap(),
    )));
    Ok(cx.string(handle.into_u64().to_string()))
}

fn logins_sync(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let handle = cx.argument::<JsString>(0)?.value().parse::<u64>().unwrap();
    let key_id = cx.argument::<JsString>(1)?.value();
    let access_token = cx.argument::<JsString>(2)?.value();
    let sync_key = cx.argument::<JsString>(3)?.value();

    PASSWORDS
        .get_mut_u64(handle, |state| -> Result<_, HandleError> {
            state
                .lock()
                .unwrap()
                .sync(
                    &sync15::Sync15StorageClientInit {
                        key_id,
                        access_token,
                        tokenserver_url: url::Url::parse(TOKENSERVER_URL).unwrap(),
                    },
                    &sync15::KeyBundle::from_ksync_base64(sync_key.as_str()).unwrap(),
                )
                .unwrap();
            Ok(())
        })
        .unwrap();
    Ok(cx.undefined())
}

fn logins_list(mut cx: FunctionContext) -> JsResult<JsString> {
    let handle = cx.argument::<JsString>(0)?.value().parse::<u64>().unwrap();
    let json = PASSWORDS
        .get_mut_u64(handle, |state| -> Result<_, HandleError> {
            let all_passwords = state.lock().unwrap().list().unwrap();
            let json = serde_json::to_string(&all_passwords).unwrap();
            Ok(json)
        })
        .unwrap();
    Ok(cx.string(json))
}

register_module!(mut cx, {
    cx.export_function("fxaNew", fxa_new)?;
    cx.export_function("fxaBeginOAuthFlow", fxa_begin_oauth_flow)?;
    cx.export_function("fxaCompleteOAuthFlow", fxa_complete_oauth_flow)?;
    cx.export_function("fxaGetAccessToken", fxa_get_access_token)?;

    cx.export_function("loginsNew", logins_new)?;
    cx.export_function("loginsSync", logins_sync)?;
    cx.export_function("loginsList", logins_list)?;
    Ok(())
});
