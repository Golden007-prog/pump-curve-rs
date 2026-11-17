//! Property-style tests: thousands of random curve states and trades from a fixed-seed generator (no dependency on a
//! property-testing crate, and the same cases on every run).

use pump_curve::{max_sol_cost, min_sol_output, BondingCurve, Error, Fees, Pubkey};

/// xorshift64*: small, deterministic, good enough to spread test cases.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// A standard curve somewhere along its life, reached by real quoted buys from launch.
fn random_curve(rng: &mut Rng) -> BondingCurve {
    let mut c = BondingCurve::fresh(Pubkey::new([1; 32]));
    for _ in 0..rng.below(4) {
        let spend = 10_000_000 + rng.below(30_000_000_000);
        if let Ok(q) = c.buy_exact_sol_in(spend, Fees::ZERO) {
            c.apply_buy(&q).unwrap();
        }
        if c.complete {
            return BondingCurve::fresh(Pubkey::new([1; 32]));
        }
    }
    c
}

fn random_fees(rng: &mut Rng) -> Fees {
    Fees::new(rng.below(300), rng.below(300))
}

#[test]
fn buy_then_sell_never_returns_more_sol_than_spent() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut checked = 0;
    for _ in 0..5_000 {
        let mut c = random_curve(&mut rng);
        let fees = random_fees(&mut rng);
        let spend = 2 + rng.below(50_000_000_000);
        let bought = match c.buy_exact_sol_in(spend, fees) {
            Ok(q) => q,
            Err(Error::AmountTooSmall) => continue,
            Err(e) => panic!("{e}"),
        };
        assert!(bought.total_sol <= spend, "spent more than the budget");
        let k_before = c.virtual_sol_reserves as u128 * c.virtual_token_reserves as u128;
        c.apply_buy(&bought).unwrap();
        let k_after = c.virtual_sol_reserves as u128 * c.virtual_token_reserves as u128;
        assert!(k_after >= k_before, "a buy must never shrink the constant product");
        if c.complete {
            continue;
        }
        match c.sell_exact_tokens_in(bought.tokens_out, fees) {
            Ok(sold) => {
                assert!(sold.sol_out <= bought.total_sol, "round trip made money: {bought:?} {sold:?}");
                assert!(sold.sol_from_curve <= bought.sol_to_curve, "curve paid back more than it took");
                checked += 1;
            }
            Err(Error::AmountTooSmall) => {}
            Err(e) => panic!("{e}"),
        }
    }
    assert!(checked > 4_000, "only {checked} round trips checked");
}

#[test]
fn the_tokens_a_budget_buys_never_cost_more_than_the_budget() {
    let mut rng = Rng(42);
    for _ in 0..5_000 {
        let c = random_curve(&mut rng);
        let fees = random_fees(&mut rng);
        let spend = 2 + rng.below(20_000_000_000);
        let Ok(q) = c.buy_exact_sol_in(spend, fees) else { continue };
        if q.tokens_out == c.real_token_reserves {
            continue; // capped: the cost is lower, not comparable
        }
        let exact = c.buy_exact_tokens_out(q.tokens_out, fees).unwrap();
        assert!(exact.sol_to_curve <= q.sol_to_curve, "{q:?} vs {exact:?}");
        assert!(exact.total_sol <= q.total_sol, "{q:?} vs {exact:?}");
    }
}

#[test]
fn more_sol_never_buys_fewer_tokens() {
    let mut rng = Rng(7);
    for _ in 0..2_000 {
        let c = random_curve(&mut rng);
        let fees = random_fees(&mut rng);
        let a = 1_000 + rng.below(10_000_000_000);
        let b = a + rng.below(1_000_000_000);
        let (Ok(qa), Ok(qb)) = (c.buy_exact_sol_in(a, fees), c.buy_exact_sol_in(b, fees)) else { continue };
        assert!(qb.tokens_out >= qa.tokens_out);
    }
}

#[test]
fn slippage_limits_bracket_the_quote() {
    let mut rng = Rng(99);
    for _ in 0..5_000 {
        let amount = rng.next() >> 2;
        let bps = rng.below(10_001);
        let hi = max_sol_cost(amount, bps).unwrap();
        let lo = min_sol_output(amount, bps).unwrap();
        assert!(lo <= amount && amount <= hi);
    }
}
