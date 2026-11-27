use crate::state::core::AppState;
use crate::{
    cli::{self, Cancel},
    model::{Cli, MfaLogin},
    profiles::Profiles,
};

pub(super) async fn prepare_region(
    state: &AppState,
    cli: &Cli,
    login: &MfaLogin,
    written: &Profiles,
    cancel: &Cancel,
) -> Result<bool, ()> {
    if written.region(&login.target).is_some() || !written.target_allowed(&login.target) {
        return Ok(false);
    }
    let Some(region) = login
        .region
        .as_deref()
        .or_else(|| written.region(&login.source))
    else {
        return Ok(false);
    };
    cli::set_region(cli, &state.files, &login.target, region, cancel)
        .await
        .map_err(|_| ())?;
    Ok(true)
}
