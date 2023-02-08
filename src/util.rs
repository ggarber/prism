pub fn parse_channel(path: &str) -> Result<String, anyhow::Error> {
    let tokens: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if (tokens.len() != 2) || (tokens[0] != "channels") {
        anyhow::bail!("invalid path")
    }
    Ok(tokens[1].to_owned())
}
