// Generated from tests/fixtures/trade_events.json (pump.fun TradeEvents read from mainnet).
// (ix_name, is_buy, sol_amount, token_amount, post virtual_sol, post virtual_token, fee_bps, fee, creator_fee_bps, creator_fee, signature)
pub type Trade = (&'static str, bool, u64, u64, u64, u64, u64, u64, u64, u64, &'static str);
pub const TRADES: &[Trade] = &[
    ("buy_exact_sol_in", true, 412946, 4904249319, 52062143886, 618299552141963, 95, 3923, 30, 1239, "2junahrKwT9LXsZmd6i3mXGz2MuuUvdzAhhEcShYbS8tPSTr9ymRzBaDRXYd96XqVPSrEcSaqz5aBFDfVrre1jG"),
    ("buy_exact_quote_in", true, 98765431, 2626464365402, 34841242858, 923905043776996, 95, 938272, 30, 296297, "2EpBdFhyVuEgXQygBoqpFWaVNXC9wQKYuXCWAezFJuzzS9XAbqzcYNQA2LARByyVoG6jaoM11xGNdgrhU2iGBy27"),
    ("buy_exact_quote_in", true, 139456678, 739932070499, 77960211152, 412902940357547, 95, 1324839, 30, 418371, "28HmhiDCrJnUPm89Hz595kt7VJUE7hrYFXfWNwz8ud3f1RtSNkjtPeQvj5ebPvcgu65kkSruPJdZWc3ZpTonFhxm"),
    ("sell", false, 311250055, 4439302165779, 47351657010, 679807257730163, 95, 2956876, 30, 933751, "3TzCoGBCtH73XhwjcHAZVsDLPMAUXPUiBAKXSDyZrwaWd2BCAGnpFj7Xvrop8tT4y7urPh46rR3Ubb622zEbtuyF"),
    ("sell", false, 805801856, 11228125624119, 47662907065, 675367955564384, 95, 7655118, 30, 2417406, "oCwn9Znk26d68LsXvMdNDDvJs1e5iYivfzY828E8HbZobHbqMvc57HSWKuwnVLfJYwmvUKSZoR1Uu2kcy2yShNd"),
    ("buy_exact_sol_in", true, 1350544, 8115823159, 73190112691, 439813505266308, 95, 12831, 30, 4052, "3mrFSv3NxjzJbSPEdGnxsTV84NUQV9oBw2WVEPU3x7Y33iSYiXwAvYSiTo8Nt7G2WnUgb36NDCYhm81F35DismJe"),
    ("sell", false, 253524532, 8460798097554, 30930876608, 1040707653627750, 95, 2408484, 30, 760574, "4c1VBwpVKYbEEUmtvY6eMaF7w2MKnAzaFv7uw5X8yzjKF3ibMHk3f5FuohyoJcydGRMhf1rGGfWGjcoRk2Y15dAk"),
    ("sell", false, 312309605, 4223390944339, 48633162597, 661894032405586, 95, 2966942, 30, 936929, "2i8L2QyjnfnUdWzXKiAAaYmuy7hh71ZWsMpPF8K5rBmi311x3DyZ8aqGp1L8wZbESDAXVW88eTB2HA1Buynw7or9"),
    ("sell", false, 374326819, 4991580751895, 48945472202, 657670641461247, 95, 3556105, 30, 1122981, "5NxPJdCw3A9XktaHygpnvNN7PEyLpo5AFnRgLEYkNjDcVKXTdFHunfVdZJNJ2KLdZ4BnohrUs5KaiHcuFnVb4HLh"),
];
