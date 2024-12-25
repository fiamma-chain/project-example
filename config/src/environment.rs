pub enum Environment {
    Local,
    Dev,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Dev => "dev",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "dev" => Ok(Self::Dev),
            other => Err(format!(
                "{} is not a supported environment. Use either `local`、`dev`、`test` or `production`.",
                other
            )),
        }
    }
}
