//! The versions of the Studio settings that this server reads. A Studio release
//! that changes the shape of the settings adds a row here and a migration in
//! [`Studio::from_value`](crate::studio::Studio::from_value).

pub(crate) const SERVER: &str = env!("CARGO_PKG_VERSION");

/// `server` is the first `govee-server` that writes `studio`.
#[derive(Debug)]
pub(crate) struct Row {
    pub(crate) studio: &'static str,
    pub(crate) server: &'static str,
}

/// The last row is the version that this server writes.
pub(crate) const TABLE: &[Row] = &[Row {
    studio: "0.1.0",
    server: "0.1.0",
}];

pub(crate) fn reads(studio: &str) -> bool {
    TABLE.iter().any(|row| row.studio == studio)
}

pub(crate) fn refusal(studio: &str, server: Option<&str>) -> String {
    let latest = TABLE.last().map_or("0.0.0", |row| row.studio);
    match server {
        Some(server) if newer_server(server) => format!(
            "this file comes from govee-server {server}, which is newer than this one \
             ({SERVER}). Update govee-server, then import the file again"
        ),
        _ if newer(studio, latest) => format!(
            "these settings come from Studio {studio}, which is newer than this govee-server \
             ({SERVER}). Update govee-server, then try again"
        ),
        _ => {
            let read: Vec<String> = TABLE
                .iter()
                .map(|row| format!("{} (govee-server {} and later)", row.studio, row.server))
                .collect();
            format!(
                "govee-server {SERVER} cannot read the settings of Studio {studio}. It reads \
                 the settings of Studio {}",
                read.join(", ")
            )
        }
    }
}

pub(crate) fn newer_server(server: &str) -> bool {
    newer(server, SERVER)
}

/// A part that is not a number counts as 0.
fn newer(a: &str, b: &str) -> bool {
    let parts = |version: &str| -> [u64; 3] {
        let mut parts = version
            .split(['.', '-', '+'])
            .map(|part| part.parse().unwrap_or(0));
        [(); 3].map(|()| parts.next().unwrap_or(0))
    };
    parts(a) > parts(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds_the_version_that_the_server_writes() {
        assert!(
            TABLE
                .last()
                .is_some_and(|row| row.studio == crate::studio::VERSION)
        );
        assert!(TABLE.iter().all(|row| !newer(row.server, SERVER)));
    }

    #[test]
    fn a_newer_release_is_named_in_the_refusal() {
        assert!(newer("0.10.0", "0.9.3"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(refusal("0.9.0", Some("99.0.0")).contains("Update govee-server"));
        assert!(refusal("9.0.0", None).contains("Update govee-server"));
        assert!(refusal("0.0.1", None).contains("cannot read"));
    }
}
