use super::types::{Region, Sector};

pub fn region_label(region: Region) -> &'static str {
    match region {
        Region::NorthAmerica => "North America",
        Region::SouthAmerica => "South America",
        Region::Europe => "Europe",
        Region::AsiaPacific => "Asia-Pacific",
        Region::MiddleEastAfrica => "Middle East & Africa",
    }
}

pub fn sector_label(sector: Sector) -> &'static str {
    match sector {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_stable() {
        assert_eq!(region_label(Region::Europe), "Europe");
        assert_eq!(
            sector_label(Sector::ConsumerDiscretionary),
            "Consumer Discretionary"
        );
    }
}
