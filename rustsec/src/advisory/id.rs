//! Advisory identifiers

use super::date::{YEAR_MAX, YEAR_MIN};
use crate::error::{Error, ErrorKind};
use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{self, Display},
    str::FromStr,
};

/// Placeholder advisory name: shouldn't be used until an ID is assigned
pub const PLACEHOLDER: &str = "RUSTSEC-0000-0000";

/// An identifier for an individual advisory
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Id {
    /// An autodetected identifier kind
    kind: Kind,

    /// Year this vulnerability was published
    year: Option<u32>,

    /// The actual string representing the identifier
    string: String,
}

impl Id {
    /// Get a string reference to this advisory ID
    pub fn as_str(&self) -> &str {
        self.string.as_ref()
    }

    /// Get the advisory kind for this advisory
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Is this advisory ID the `RUSTSEC-0000-0000` placeholder ID?
    pub fn is_placeholder(&self) -> bool {
        self.string == PLACEHOLDER
    }

    /// Is this advisory ID a RUSTSEC advisory?
    pub fn is_rustsec(&self) -> bool {
        self.kind == Kind::RustSec
    }

    /// Is this advisory ID a CVE?
    pub fn is_cve(&self) -> bool {
        self.kind == Kind::Cve
    }

    /// Is this advisory ID a GHSA?
    pub fn is_ghsa(&self) -> bool {
        self.kind == Kind::Ghsa
    }

    /// Is this an unknown kind of advisory ID?
    pub fn is_other(&self) -> bool {
        self.kind == Kind::Other
    }

    /// Get the year this vulnerability was published (if known)
    pub fn year(&self) -> Option<u32> {
        self.year
    }

    /// Get the numerical part of this advisory (if available).
    ///
    /// This corresponds to the numbers on the right side of the ID.
    pub fn numerical_part(&self) -> Option<u32> {
        if self.is_placeholder() {
            return None;
        }

        self.string
            .split('-')
            .last()
            .and_then(|s| str::parse(s).ok())
    }

    /// Get a URL to a web page with more information on this advisory
    // TODO(tarcieri): look up GHSA URLs via the GraphQL API?
    // <https://developer.github.com/v4/object/securityadvisory/>
    pub fn url(&self) -> Option<String> {
        match self.kind {
            Kind::RustSec => {
                if self.is_placeholder() {
                    None
                } else {
                    Some(format!("https://rustsec.org/advisories/{}", &self.string))
                }
            }
            Kind::Cve => Some(format!(
                "https://cve.mitre.org/cgi-bin/cvename.cgi?name={}",
                &self.string
            )),
            Kind::Ghsa => Some(format!("https://github.com/advisories/{}", &self.string)),
            Kind::Talos => Some(format!(
                "https://www.talosintelligence.com/reports/{}",
                &self.string
            )),
            _ => None,
        }
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Default for Id {
    fn default() -> Id {
        Id {
            kind: Kind::RustSec,
            year: None,
            string: PLACEHOLDER.into(),
        }
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Id {
    type Err = Error;

    /// Create an `Id` from the given string
    fn from_str(advisory_id: &str) -> Result<Self, Error> {
        if advisory_id == PLACEHOLDER {
            return Ok(Id::default());
        }

        let kind = Kind::detect(advisory_id);

        // Ensure known advisory types are well-formed
        let year = match kind {
            Kind::RustSec | Kind::Cve | Kind::Talos => Some(parse_year(advisory_id)?),
            _ => None,
        };

        Ok(Self {
            kind,
            year,
            string: advisory_id.into(),
        })
    }
}

impl Serialize for Id {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.string)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::from_str(&String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// Known kinds of advisory IDs
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Kind {
    /// Our advisory namespace
    RustSec,

    /// Common Vulnerabilities and Exposures
    Cve,

    /// GitHub Security Advisory
    Ghsa,

    /// Cisco Talos identifiers
    Talos,

    /// Other types of advisory identifiers we don't know about
    Other,
}

impl Kind {
    /// Detect the identifier kind for the given string
    pub fn detect(string: &str) -> Self {
        if string.starts_with("RUSTSEC-") {
            Kind::RustSec
        } else if string.starts_with("CVE-") {
            Kind::Cve
        } else if string.starts_with("TALOS-") {
            Kind::Talos
        } else if string.starts_with("GHSA-") {
            Kind::Ghsa
        } else {
            Kind::Other
        }
    }
}

/// Parse the year from an advisory identifier
fn parse_year(advisory_id: &str) -> Result<u32, Error> {
    let mut parts = advisory_id.split('-');
    parts.next().unwrap();

    let year = match parts.next().unwrap().parse::<u32>() {
        Ok(n) => match n {
            YEAR_MIN..=YEAR_MAX => n,
            _ => fail!(
                ErrorKind::Parse,
                "out-of-range year in advisory ID: {}",
                advisory_id
            ),
        },
        _ => fail!(
            ErrorKind::Parse,
            "malformed year in advisory ID: {}",
            advisory_id
        ),
    };

    if let Some(num) = parts.next() {
        if num.parse::<u32>().is_err() {
            fail!(ErrorKind::Parse, "malformed advisory ID: {}", advisory_id);
        }
    } else {
        fail!(ErrorKind::Parse, "incomplete advisory ID: {}", advisory_id);
    }

    if parts.next().is_some() {
        fail!(ErrorKind::Parse, "malformed advisory ID: {}", advisory_id);
    }

    Ok(year)
}

#[cfg(test)]
mod tests {
    use super::{Id, Kind, PLACEHOLDER};

    const EXAMPLE_RUSTSEC_ID: &str = "RUSTSEC-2018-0001";
    const EXAMPLE_CVE_ID: &str = "CVE-2017-1000168";
    const EXAMPLE_GHSA_ID: &str = "GHSA-4mmc-49vf-jmcp";
    const EXAMPLE_TALOS_ID: &str = "TALOS-2017-0468";
    const EXAMPLE_UNKNOWN_ID: &str = "Anonymous-42";

    #[test]
    fn rustsec_id_test() {
        let rustsec_id = EXAMPLE_RUSTSEC_ID.parse::<Id>().unwrap();
        assert!(rustsec_id.is_rustsec());
        assert_eq!(rustsec_id.year().unwrap(), 2018);
        assert_eq!(
            rustsec_id.url().unwrap(),
            "https://rustsec.org/advisories/RUSTSEC-2018-0001"
        );
        assert_eq!(rustsec_id.numerical_part().unwrap(), 0001);
    }

    // The RUSTSEC-0000-0000 ID is a placeholder we need to treat as valid
    #[test]
    fn rustsec_0000_0000_test() {
        let rustsec_id = PLACEHOLDER.parse::<Id>().unwrap();
        assert!(rustsec_id.is_rustsec());
        assert!(rustsec_id.year().is_none());
        assert!(rustsec_id.url().is_none());
        assert!(rustsec_id.numerical_part().is_none());
    }

    #[test]
    fn cve_id_test() {
        let cve_id = EXAMPLE_CVE_ID.parse::<Id>().unwrap();
        assert!(cve_id.is_cve());
        assert_eq!(cve_id.year().unwrap(), 2017);
        assert_eq!(
            cve_id.url().unwrap(),
            "https://cve.mitre.org/cgi-bin/cvename.cgi?name=CVE-2017-1000168"
        );
        assert_eq!(cve_id.numerical_part().unwrap(), 1000168);
    }

    #[test]
    fn ghsa_id_test() {
        let ghsa_id = EXAMPLE_GHSA_ID.parse::<Id>().unwrap();
        assert!(ghsa_id.is_ghsa());
        assert!(ghsa_id.year().is_none());
        assert_eq!(
            ghsa_id.url().unwrap(),
            "https://github.com/advisories/GHSA-4mmc-49vf-jmcp"
        );
        assert!(ghsa_id.numerical_part().is_none());
    }

    #[test]
    fn talos_id_test() {
        let talos_id = EXAMPLE_TALOS_ID.parse::<Id>().unwrap();
        assert_eq!(talos_id.kind(), Kind::Talos);
        assert_eq!(talos_id.year().unwrap(), 2017);
        assert_eq!(
            talos_id.url().unwrap(),
            "https://www.talosintelligence.com/reports/TALOS-2017-0468"
        );
        assert_eq!(talos_id.numerical_part().unwrap(), 0468);
    }

    #[test]
    fn other_id_test() {
        let other_id = EXAMPLE_UNKNOWN_ID.parse::<Id>().unwrap();
        assert!(other_id.is_other());
        assert!(other_id.year().is_none());
        assert!(other_id.url().is_none());
        assert_eq!(other_id.numerical_part().unwrap(), 42);
    }
}
