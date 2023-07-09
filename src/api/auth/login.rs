use axum::{
    extract::State,
    response::{Html, Response},
    Form,
};
use email_address::EmailAddress;
use http::{HeaderMap, HeaderValue, StatusCode};
use serde::Deserialize;
use sqlx::PgPool;
use tower_cookies::Cookies;

use crate::{
    api::auth::make_jwt_token, data::app_state::AppState, login_partial, utils::ToServerError,
};

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

pub async fn login(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> Result<Html<String>, StatusCode> {
    tracing::debug!(
        "request login for user ({}) with password ({}).",
        form.username,
        form.password
    );

    match get_password_hash_from_username_or_email(&form.username, &state.pool)
        .await
        .server_error()?
    {
        Some((user_id, stored_password_hash)) => {
            tracing::debug!("found user ({}) id ({}). ", form.username, user_id);
            let passwords_match =
                bcrypt::verify(form.password, &stored_password_hash).server_error()?;
            if passwords_match {
                // TODO! redirect to "/" with credentails

                tracing::debug!("password correct: id: {}.", user_id);

                make_jwt_token(user_id, form.username, &cookies, state)
                    .await
                    .server_error()?;

                tracing::debug!("created tokens: id: {}.", user_id);

                Ok(Html(
                    "loading...\n<meta http-equiv=\"refresh\" content=\"0\" />".to_owned(),
                ))
            } else {
                tracing::debug!(
                    "login atempt for user ({}) failed wrong password",
                    form.username
                );
                Ok(Html(login_partial("Wrong username or password")))
            }
        }
        None => {
            tracing::debug!("no user ({}) found", form.username);

            Ok(Html(login_partial("Wrong username or password")))
        }
    }
}

async fn get_password_hash_from_username_or_email(
    username: &str,
    pool: &PgPool,
) -> anyhow::Result<Option<(i32, String)>> {
    if EmailAddress::is_valid(username) {
        let rec = sqlx::query!(
            "SELECT id, password_hash FROM users WHERE email = $1",
            username
        )
        .fetch_one(pool)
        .await;

        match rec {
            Ok(rec) => Ok(Some((rec.id, rec.password_hash))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e)?,
        }
    } else {
        let rec = sqlx::query!(
            "SELECT id, password_hash FROM users WHERE username = $1",
            username
        )
        .fetch_one(pool)
        .await;

        match rec {
            Ok(rec) => Ok(Some((rec.id, rec.password_hash))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e)?,
        }
    }
}
