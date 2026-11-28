//! What can go wrong between a scene name and the scene's parameters.

use std::path::PathBuf;

use crate::error::Category;

/// A scene that could not be found, read or fetched.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// No library for the model is in the cache. Fetch it first.
    #[error("no scene library is cached for `{sku}`: fetch it first")]
    NotFetched {
        /// The model.
        sku: String,
    },

    /// The name or the code matches no scene of the model.
    #[error("`{query}` matches no scene of `{sku}`")]
    Unknown {
        /// The model.
        sku: String,
        /// What was asked for.
        query: String,
    },

    /// The name matches more than one scene. The full names tell them apart.
    #[error("`{query}` matches more than one scene of `{sku}`: {}", .matches.join(", "))]
    Ambiguous {
        /// The model.
        sku: String,
        /// What was asked for.
        query: String,
        /// The full name of each match.
        matches: Vec<String>,
    },

    /// The cached library or the fetched answer does not read as a library.
    #[error("scene library `{path}`: {reason}")]
    Unreadable {
        /// The file, or the address that answered.
        path: PathBuf,
        /// What is wrong with it.
        reason: String,
    },

    /// A model name that cannot name a file.
    #[error("`{sku}` is not a model name")]
    BadSku {
        /// What was given.
        sku: String,
    },

    /// The library could not be fetched.
    #[error("the scene library of `{sku}` could not be fetched: {reason}")]
    Fetch {
        /// The model.
        sku: String,
        /// What failed.
        reason: String,
    },
}

impl Error {
    /// A stable, language-neutral identifier, in the namespace of
    /// [`crate::Error::code`].
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFetched { .. } => "scene_list_missing",
            Self::Unknown { .. } => "unknown_scene",
            Self::Ambiguous { .. } => "ambiguous_scene",
            Self::Unreadable { .. } => "scene_list_unreadable",
            Self::BadSku { .. } => "bad_sku",
            Self::Fetch { .. } => "scene_fetch_failed",
        }
    }

    /// The family this failure belongs to.
    #[must_use]
    pub fn category(&self) -> Category {
        match self {
            Self::NotFetched { .. } | Self::Unreadable { .. } => Category::Config,
            Self::Unknown { .. } | Self::Ambiguous { .. } | Self::BadSku { .. } => Category::Codec,
            Self::Fetch { .. } => Category::Transport,
        }
    }
}
