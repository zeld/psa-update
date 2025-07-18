use console::style;

use dialoguer::{Confirm, Input, Select};

use anyhow::{Error, Result};

use crate::psa;

pub fn confirm(message: &str) -> Result<bool, Error> {
    Ok(Confirm::new().with_prompt(message).interact()?)
}

pub fn prompt(message: &str) -> Result<String, Error> {
    Ok(Input::new().with_prompt(message).interact_text()?)
}

fn select(message: &str, items: &[&str]) -> Result<Option<usize>, Error> {
    let index = Select::new()
        .items(items)
        .default(0)
        .with_prompt(message)
        .interact_opt()?;
    Ok(index)
}

pub fn select_map() -> Result<Option<&'static str>, Error> {
    let items: Vec<&str> = psa::MAPS.iter().map(|m| m.get_name()).collect();
    let map_code = select("Check for a map update (hit ESC to skip)", &items)?
        .map(|index| psa::MAPS[index].get_code());
    Ok(map_code)
}

pub fn warn(message: &str) {
    println!("{} {}", style("[warning]").yellow(), message);
}
