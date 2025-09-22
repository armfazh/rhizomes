// Copyright (c) 2025 Armando Faz Hernandez.
// SPDX-License-Identifier: MPL-2.0

//! Faster algorithms for polynomials in the Lagrange basis.
//!
//! Reference:
//! Faz-Hernandez, "Rhizomes and the Roots of Efficiency -- Improving Prio."
//! <https://doi.org/10.1007/978-3-032-06754-8_16>

use crate::{
    field::{FieldElement, NttFriendlyFieldElement},
    fp::log2,
    ntt::{ntt, ntt_inv_finish, ntt_star},
};

/// Returns the element `1/(2^n)` on `F`.
#[inline]
fn half_power<F: FieldElement>(n: usize) -> F {
    let half = F::half();
    let mut x = F::one();
    for _ in 0..n {
        x *= half
    }
    x
}

/// Returns the element `1/n` on `F`, where `n` must be a power of two.
#[inline]
pub(crate) fn inv_pow2<F: FieldElement>(n: usize) -> F {
    let log2_n = usize::try_from(log2(n as u128)).unwrap();
    assert_eq!(n, 1 << log2_n);

    half_power(log2_n)
}

fn term_w<F: FieldElement>(m: usize, i: usize, roots: &[F]) -> F {
    let mut w = F::one();
    for j in 0..=m {
        if i != j {
            w *= roots[i] - roots[j];
        }
    }
    w.inv()
}

/// Extends dimension by one, keeping the degree of polynomial unchanged.
pub fn extend_dimension_one<F: FieldElement>(values: &[F], roots: &[F]) -> F {
    let k = values.len();
    let n = roots.len();
    assert!(k < n);
    let mut y = F::zero();
    if k == n - 1 {
        // Special case.
        for (yi, roots_j) in values.iter().zip(roots.iter().cycle().skip(1 + (n >> 1))) {
            y += *yi * *roots_j;
        }
    } else if k < n {
        // General case.
        for (i, yi) in values.iter().enumerate() {
            y += *yi * term_w(k, i, roots);
        }
        y *= -term_w(k, k, roots).inv();
    }
    y
}

/// Given (a0, ..., aN, b0, ..., bN) permutes the input in place as (a0,b0, ..., aN,bN).
/// https://rosettacode.org/wiki/Perfect_shuffle
fn perfect_shuffle<T>(input: &mut [T]) {
    let n = input.len();
    if n >= 4 {
        let (a, b) = input.split_at_mut(n >> 1);
        let (_, a1) = a.split_at_mut(n >> 2);
        let (b0, _) = b.split_at_mut(n >> 2);
        a1.swap_with_slice(b0);
        perfect_shuffle(a);
        perfect_shuffle(b);
    }
}

/// Extends dimension by double, keeping the degree of polynomial unchanged.
pub fn extend_dimension_double<F: NttFriendlyFieldElement>(values: &mut [F], n: usize) {
    assert!(2 * n <= values.len());
    let n_inv = inv_pow2(n);
    let mut tmp = vec![F::zero(); n];
    ntt(&mut tmp, &values[..n], n).unwrap();
    ntt_inv_finish(&mut tmp, n, n_inv);
    ntt_star(&mut values[n..2 * n], &tmp, n).unwrap();
    perfect_shuffle(&mut values[..2 * n]);
}

/// Evaluates a polynomial given in the Lagrange basis.
///
/// This is the implementation of Algorithm 6.
#[allow(dead_code)]
pub fn poly_eval_rhizomes<F: NttFriendlyFieldElement>(poly: &[F], roots: &[F], x: &F) -> F {
    let n = poly.len();
    let mut l = F::one();
    let mut u = poly[0];
    let mut d = roots[0] - *x;
    for (yi, wn_i) in poly[1..n].iter().zip(&roots[1..n]) {
        l *= d;
        d = *wn_i - *x;
        u = u * d + l * *wn_i * *yi;
    }

    for wn_i in &roots[n..] {
        u *= *wn_i - *x;
    }

    if roots.len() > 1 {
        let num_roots_inv = -inv_pow2::<F>(roots.len());
        u *= num_roots_inv;
    }

    u
}

/// Evaluates all polynomials given in the Lagrange basis.
///
/// This is the implementation of Algorithm 7.
pub fn poly_eval_rhizomes_batched<F: FieldElement>(
    polynomials: &[Vec<F>],
    roots: &[F],
    x: F,
) -> Vec<F> {
    let mut l = F::one();
    let mut u = Vec::with_capacity(polynomials.len());
    u.extend(polynomials.iter().map(|poly| poly[0]));
    let mut d = roots[0] - x;
    for (i, wn_i) in (1..).zip(&roots[1..]) {
        l *= d;
        d = *wn_i - x;
        let t = l * *wn_i;
        for (u_j, poly) in u.iter_mut().zip(polynomials) {
            *u_j *= d;
            if let Some(yi) = poly.get(i) {
                *u_j += t * *yi;
            }
        }
    }

    if roots.len() > 1 {
        let num_roots_inv = -inv_pow2::<F>(roots.len());
        u.iter_mut().for_each(|u_j| *u_j *= num_roots_inv);
    }

    u
}

/// Evaluates a polynomial given in the Lagrange basis at multiple values.
///
/// This is an implementation of Algorithm 6 with multiple evaluation points.
pub fn poly_multieval_rhizomes_batched<F: NttFriendlyFieldElement>(
    output_x: &mut [F],
    poly: &[F],
    roots: &[F],
) {
    let n = poly.len();
    let z = poly[..n]
        .iter()
        .zip(&roots[..n])
        .map(|(yi, wn_i)| *yi * *wn_i)
        .collect::<Vec<_>>();
    let num_roots_inv = -inv_pow2::<F>(roots.len());

    for x_j in output_x.iter_mut() {
        let mut l = F::one();
        let mut u = poly[0];
        let mut d = roots[0] - *x_j;
        for (zi, wn_i) in z[1..n].iter().zip(&roots[1..n]) {
            l *= d;
            d = *wn_i - *x_j;
            u = u * d + l * *zi;
        }

        for wn_i in &roots[n..] {
            u *= *wn_i - *x_j;
        }

        if roots.len() > 1 {
            u *= num_roots_inv;
        }

        *x_j = u
    }
}

/// Generates the powers of the primitive n-th root of unity.
///
/// Returns
///   roots\[i\] = w_n^i for 0 ≤ i < n,
/// where
///   w_n is the primitive n-th root of unity in `F`, and
///   n must be a power of two.
pub fn nth_root_powers<F: NttFriendlyFieldElement>(n: usize) -> Vec<F> {
    let log2_n = usize::try_from(log2(n as u128)).unwrap();
    assert_eq!(n, 1 << log2_n);

    let mut roots = vec![F::zero(); n];
    roots[0] = F::one();
    if n > 1 {
        roots[1] = -F::one();
        for i in 2..=log2_n {
            let mid = 1 << (i - 1);
            // Due to w_{2n}^{2j} = w_{n}^j
            for j in (1..mid).rev() {
                roots[j << 1] = roots[j]
            }

            let wn = F::root(i).unwrap();
            roots[1] = wn;
            roots[1 + mid] = -wn;

            // Due to w_{n}^{j} = -w_{n}^{j+n/2}
            for j in (3..mid).step_by(2) {
                roots[j] = wn * roots[j - 1];
                roots[j + mid] = -roots[j]
            }
        }
    }

    roots
}

#[cfg(test)]
mod test_methods {
    use crate::{
        field::NttFriendlyFieldElement,
        fp::log2,
        ntt::{ntt, ntt_inv_finish},
        polynomial::poly_eval,
    };

    /// Evaluates a polynomial given in Lagrange basis.
    ///
    /// Converts the polynomial from the Lagrange to monomial basis (with inverse NTT),
    /// and performs evaluation using the Horner's method.
    pub(crate) fn poly_eval_monomial<F: NttFriendlyFieldElement>(
        points: &[F],
        eval_at: F,
        tmp_coeffs: &mut [F],
        size_inv: F,
    ) -> F {
        ntt(tmp_coeffs, points, points.len()).unwrap();
        ntt_inv_finish(tmp_coeffs, points.len(), size_inv);
        poly_eval(&tmp_coeffs[..points.len()], eval_at)
    }

    /// Generates the powers of the primitive n-th root of unity.
    ///
    /// Returns
    ///   roots\[i\] = w_n^i for 0 ≤ i < n,
    /// where
    ///   w_n is the primitive n-th root of unity in `F`, and
    ///   n must be a power of two.
    ///
    /// This is the iterative method.
    pub(crate) fn nth_root_powers_slow<F: NttFriendlyFieldElement>(n: usize) -> Vec<F> {
        let log2_n = usize::try_from(log2(n as u128)).unwrap();
        let wn = F::root(log2_n).unwrap();
        core::iter::successors(Some(F::one()), |&x| Some(x * wn))
            .take(n)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        field::{Field64 as Fp, FieldElement, FieldElementWithInteger},
        rhizomes::test_methods::{nth_root_powers_slow, poly_eval_monomial},
        rhizomes::{nth_root_powers, poly_eval_rhizomes, poly_eval_rhizomes_batched},
    };

    #[test]
    fn test_nth_root_powers() {
        for i in 0..8 {
            assert_eq!(
                nth_root_powers::<Fp>(1 << i),
                nth_root_powers_slow::<Fp>(1 << i)
            );
        }
    }

    #[test]
    fn test_poly_eval_rhizomes() {
        for size in (1usize..100).step_by(10) {
            let n: usize = size.next_power_of_two();
            let n_inv =
                Fp::from(<Fp as FieldElementWithInteger>::Integer::try_from(n).unwrap()).inv();
            let values = Fp::random_vector(n);
            let x = Fp::random_vector(1)[0];
            let mut ntt_mem = vec![Fp::zero(); n];

            // Evaluates a polynomial converting to the monomial basis.
            let want = poly_eval_monomial(&values, x, &mut ntt_mem, n_inv);

            // Evaluates a polynomial directly in the Lagrange basis.
            let roots = nth_root_powers(n);
            let got = poly_eval_rhizomes(&values, &roots, &x);
            assert_eq!(got, want, "n: {n} x: {x} values: {values:?}");
        }
    }

    #[test]
    fn test_poly_eval_batched_ones() {
        test_poly_eval_batched(&[1]);
        test_poly_eval_batched(&[1, 1]);
    }

    #[test]
    fn test_poly_eval_batched_powers() {
        test_poly_eval_batched(&[1, 2, 4, 16, 64]);
    }

    #[test]
    fn test_poly_eval_batched_arbitrary() {
        test_poly_eval_batched(&[1, 6, 3, 9]);
    }

    fn test_poly_eval_batched(lengths: &[usize]) {
        let sizes = lengths
            .iter()
            .map(|s| s.next_power_of_two())
            .collect::<Vec<_>>();

        let polynomials = sizes
            .iter()
            .map(|&size| Fp::random_vector(size))
            .collect::<Vec<_>>();
        let x = Fp::random_vector(1)[0];

        let &n = sizes.iter().max().unwrap();
        let n_inv = Fp::from(<Fp as FieldElementWithInteger>::Integer::try_from(n).unwrap()).inv();
        let mut ntt_mem = vec![Fp::zero(); n];
        let roots = nth_root_powers(n);

        // Evaluates several polynomials converting them to the monomial basis (iteratively).
        let want = polynomials
            .iter()
            .map(|poly| {
                let extended_poly = [poly.clone(), vec![Fp::zero(); n - poly.len()]].concat();
                poly_eval_monomial(&extended_poly, x, &mut ntt_mem, n_inv)
            })
            .collect::<Vec<_>>();

        // Evaluates several polynomials directly in the Lagrange basis (iteratively).
        let got = polynomials
            .iter()
            .map(|poly| poly_eval_rhizomes(poly, &roots, &x))
            .collect::<Vec<_>>();
        assert_eq!(got, want, "sizes: {sizes:?} x: {x} P: {polynomials:?}");

        // Simultaneouly evaluates several polynomials directly in the Lagrange basis (batched).
        let got = poly_eval_rhizomes_batched(&polynomials, &roots, x);
        assert_eq!(got, want, "sizes: {sizes:?} x: {x} P: {polynomials:?}");
    }
}
