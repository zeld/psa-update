use console::style;

use dialoguer::{Confirm, Input, Select};

use anyhow::{Error, Result};

pub fn confirm(message: &str) -> Result<bool, Error> {
    Ok(Confirm::new().with_prompt(message).interact()?)
}

pub fn prompt(message: &str) -> Result<String, Error> {
    //FIXME interact_text() should be used instead but there is currently a bug
    // on Windows that triggers an error when the user presses the Shift/AltGr keys
    // https://github.com/mitsuhiko/dialoguer/issues/128
    Ok(Input::new().with_prompt(message).interact()?)
}

pub fn select(message: &str, items: &[&str]) -> Result<Option<usize>, Error> {
    let index = Select::new()
        .items(&items)
        .default(0)
        .with_prompt(message)
        .interact_opt()?;
    Ok(index)
}

pub fn warn(message: &str) {
    println!("{} {}", style("[warning]").yellow(), message);
}
