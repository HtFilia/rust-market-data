use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Region {
    NorthAmerica,
    SouthAmerica,
    Europe,
    AsiaPacific,
    MiddleEastAfrica,
}

impl Region {
    pub const ALL: [Region; 5] = [
        Region::NorthAmerica,
        Region::SouthAmerica,
        Region::Europe,
        Region::AsiaPacific,
        Region::MiddleEastAfrica,
    ];

    pub fn prefix(self) -> &'static str {
        match self {
            Region::NorthAmerica => "NA",
            Region::SouthAmerica => "SA",
            Region::Europe => "EU",
            Region::AsiaPacific => "AP",
            Region::MiddleEastAfrica => "ME",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Region::NorthAmerica => 0,
            Region::SouthAmerica => 1,
            Region::Europe => 2,
            Region::AsiaPacific => 3,
            Region::MiddleEastAfrica => 4,
        }
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Region::NorthAmerica => "North America",
            Region::SouthAmerica => "South America",
            Region::Europe => "Europe",
            Region::AsiaPacific => "Asia Pacific",
            Region::MiddleEastAfrica => "Middle East & Africa",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sector {
    Technology,
    Financials,
    Industrials,
    Healthcare,
    ConsumerDiscretionary,
    ConsumerStaples,
    Energy,
    Utilities,
    Materials,
    RealEstate,
}

impl Sector {
    pub const ALL: [Sector; 10] = [
        Sector::Technology,
        Sector::Financials,
        Sector::Industrials,
        Sector::Healthcare,
        Sector::ConsumerDiscretionary,
        Sector::ConsumerStaples,
        Sector::Energy,
        Sector::Utilities,
        Sector::Materials,
        Sector::RealEstate,
    ];

    pub fn prefix(self) -> &'static str {
        match self {
            Sector::Technology => "TECH",
            Sector::Financials => "FIN",
            Sector::Industrials => "IND",
            Sector::Healthcare => "HLT",
            Sector::ConsumerDiscretionary => "CND",
            Sector::ConsumerStaples => "CNS",
            Sector::Energy => "ENG",
            Sector::Utilities => "UTL",
            Sector::Materials => "MAT",
            Sector::RealEstate => "REA",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Sector::Technology => 0,
            Sector::Financials => 1,
            Sector::Industrials => 2,
            Sector::Healthcare => 3,
            Sector::ConsumerDiscretionary => 4,
            Sector::ConsumerStaples => 5,
            Sector::Energy => 6,
            Sector::Utilities => 7,
            Sector::Materials => 8,
            Sector::RealEstate => 9,
        }
    }
}

impl fmt::Display for Sector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Sector::Technology => "Technology",
            Sector::Financials => "Financials",
            Sector::Industrials => "Industrials",
            Sector::Healthcare => "Healthcare",
            Sector::ConsumerDiscretionary => "Consumer Discretionary",
            Sector::ConsumerStaples => "Consumer Staples",
            Sector::Energy => "Energy",
            Sector::Utilities => "Utilities",
            Sector::Materials => "Materials",
            Sector::RealEstate => "Real Estate",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Equity {
    pub symbol: String,
    pub region: Region,
    pub sector: Sector,
}

pub fn default_equities() -> Vec<Equity> {
    const REPLICATION_PER_BUCKET: usize = 10;

    let mut equities =
        Vec::with_capacity(Region::ALL.len() * Sector::ALL.len() * REPLICATION_PER_BUCKET);
    for region in Region::ALL {
        for sector in Sector::ALL {
            for replica in 0..REPLICATION_PER_BUCKET {
                let symbol = format!(
                    "{region_prefix}{sector_prefix}{:03}",
                    replica,
                    region_prefix = region.prefix(),
                    sector_prefix = sector.prefix()
                );
                equities.push(Equity {
                    symbol,
                    region,
                    sector,
                });
            }
        }
    }

    assert_eq!(
        equities.len(),
        Region::ALL.len() * Sector::ALL.len() * REPLICATION_PER_BUCKET,
        "default equity universe size mismatch"
    );

    equities
}
