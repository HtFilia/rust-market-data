use anyhow::{Context, Result};
use nalgebra::{Cholesky, DMatrix, DVector};
use rand::rngs::StdRng;
use rand::Rng;

use crate::model::{Equity, Region, Sector};

pub struct StockUniverse {
    equities: Vec<Equity>,
    correlation: DMatrix<f64>,
    cholesky: DMatrix<f64>,
}

impl StockUniverse {
    pub fn new(equities: Vec<Equity>, rng: &mut StdRng) -> Result<Self> {
        let correlation = Self::factor_based_correlation(&equities, rng);
        let cholesky = Self::compute_cholesky(&correlation)?;
        Ok(Self {
            equities,
            correlation,
            cholesky,
        })
    }

    pub fn equities(&self) -> &[Equity] {
        &self.equities
    }

    pub fn cholesky(&self) -> &DMatrix<f64> {
        &self.cholesky
    }

    pub fn refresh(&mut self, rng: &mut StdRng) -> Result<()> {
        let candidate = Self::factor_based_correlation(&self.equities, rng);
        let blended = &self.correlation * 0.8 + candidate * 0.2;
        let renormalized = Self::renormalize(blended);
        let cholesky = Self::compute_cholesky(&renormalized)?;
        self.correlation = renormalized;
        self.cholesky = cholesky;
        Ok(())
    }

    pub fn rebuild(&mut self, rng: &mut StdRng) -> Result<()> {
        let correlation = Self::factor_based_correlation(&self.equities, rng);
        let cholesky = Self::compute_cholesky(&correlation)?;
        self.correlation = correlation;
        self.cholesky = cholesky;
        Ok(())
    }

    fn factor_based_correlation(equities: &[Equity], rng: &mut StdRng) -> DMatrix<f64> {
        let base_columns = 1 + Region::ALL.len() + Sector::ALL.len();
        let mut feature_data = Vec::with_capacity(equities.len() * (base_columns + 1));

        for equity in equities {
            let mut row = vec![0.0; base_columns + 1];
            row[0] = rng.gen_range(0.55..0.8); // global market beta

            let region_offset = 1 + equity.region.index();
            row[region_offset] = rng.gen_range(0.35..0.6);

            let sector_offset = 1 + Region::ALL.len() + equity.sector.index();
            row[sector_offset] = rng.gen_range(0.4..0.7);

            // idiosyncratic style factor to avoid perfect collinearity
            let idiosyncratic_offset = base_columns;
            row[idiosyncratic_offset] = rng.gen_range(0.05..0.12);
            feature_data.extend(row);
        }

        let feature_matrix =
            DMatrix::from_row_slice(equities.len(), base_columns + 1, &feature_data);
        let mut covariance = &feature_matrix * feature_matrix.transpose();

        for i in 0..equities.len() {
            covariance[(i, i)] += rng.gen_range(0.08..0.15);
        }

        Self::renormalize(covariance)
    }

    fn renormalize(matrix: DMatrix<f64>) -> DMatrix<f64> {
        let size = matrix.nrows();
        let mut normalized = matrix.clone();
        let mut diag = DVector::zeros(size);
        for i in 0..size {
            diag[i] = matrix[(i, i)].max(f64::EPSILON).sqrt();
        }
        for i in 0..size {
            for j in 0..size {
                normalized[(i, j)] = matrix[(i, j)] / (diag[i] * diag[j]);
            }
            normalized[(i, i)] = 1.0;
        }
        normalized
    }

    fn compute_cholesky(matrix: &DMatrix<f64>) -> Result<DMatrix<f64>> {
        Cholesky::new(matrix.clone())
            .map(|decomposition| decomposition.l().clone_owned())
            .with_context(|| "failed to compute Cholesky factor for correlation matrix")
    }
}

#[cfg(test)]
impl StockUniverse {
    pub(crate) fn correlation_matrix(&self) -> &DMatrix<f64> {
        &self.correlation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn build_sample_equities() -> Vec<Equity> {
        vec![
            Equity {
                symbol: "EQ0".into(),
                region: Region::NorthAmerica,
                sector: Sector::Technology,
            },
            Equity {
                symbol: "EQ1".into(),
                region: Region::Europe,
                sector: Sector::Financials,
            },
            Equity {
                symbol: "EQ2".into(),
                region: Region::AsiaPacific,
                sector: Sector::Energy,
            },
        ]
    }

    #[test]
    fn new_universe_has_unit_diagonal() {
        let mut rng = StdRng::seed_from_u64(7);
        let universe = StockUniverse::new(build_sample_equities(), &mut rng).expect("universe");
        let corr = universe.correlation_matrix();

        for i in 0..corr.nrows() {
            assert!(
                (corr[(i, i)] - 1.0).abs() < 1e-9,
                "diagonal not normalised: {}",
                corr[(i, i)]
            );
        }
        assert!(Cholesky::new(corr.clone()).is_some(), "matrix must be SPD");
    }

    #[test]
    fn refresh_preserves_positive_definiteness() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut universe = StockUniverse::new(build_sample_equities(), &mut rng).expect("universe");

        for _ in 0..5 {
            universe.refresh(&mut rng).expect("refresh");
            let corr = universe.correlation_matrix();
            assert!(
                Cholesky::new(corr.clone()).is_some(),
                "refreshed matrix not SPD"
            );
        }
    }

    #[test]
    fn rebuild_restarts_correlation_structure() {
        let mut rng = StdRng::seed_from_u64(123);
        let mut universe = StockUniverse::new(build_sample_equities(), &mut rng).expect("universe");
        let before = universe.correlation_matrix().clone();

        universe.rebuild(&mut rng).expect("rebuild");
        let after = universe.correlation_matrix();

        assert!(
            Cholesky::new(after.clone()).is_some(),
            "rebuilt matrix not SPD"
        );
        assert_ne!(before, *after, "rebuild should produce a distinct matrix");
    }
}
