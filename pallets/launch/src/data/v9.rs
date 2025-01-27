//! This file describes the migrations to be run at stage 10.
//!
//! The goal of this migration is to mint the testnet
//! validator airdrops.
use crate::airdrops::RawAirdropMintStage;

use time_primitives::ANLOG;

// Incentivized testnet validator rewards
pub const AIRDROPS_VALIDATORS: RawAirdropMintStage = &[
	("an8Evn8LD7svf1z8EiMVRNshLajYWEaj7jk6MgAo4jRUPF8Sv", 1_895_325 * ANLOG, None),
	("an9LCe2usU8eND2vbnHqX2TgLZaoGHS4ZHb2BxYEiNA8FL8Yw", 1_860_401 * ANLOG, None),
	("an6iVGebSszTVJmDeWs54dHExuZV7o3YChT6BfYoKTMKUTW4X", 1_577_242 * ANLOG, None),
	("an6B3CSijQCDfJ1rL4bxto2iVqfPgWLvnqdcJbaJLPWup3rap", 1_554_190 * ANLOG, None),
	("an7ifiL7Nvrj21GpVR8D272rGhfztQP6GEtfqapSDRTnUQwus", 1_371_112 * ANLOG, None),
	("anB1P8QYwiqH56GAndw995AR9RHDNMHTqWQCPXhWzRZQy4Nk2", 1_357_228 * ANLOG, None),
	("an6SjbzpP2DC6VUa53kUHEvXqsiSVBfiLV9fVQjCXLctuBYL3", 1_315_241 * ANLOG, None),
	("anB28ZvSVsdsWa5EKhZftz9HF3Z5N3oN844ijKpKKHdc4noLf", 1_274_465 * ANLOG, None),
	("an9qDsq18hdhqT9k6vjqZE51h8joDCnurZCFS3p6UKQPHKbbi", 1_119_567 * ANLOG, None),
	("anACVrV6upon561F4feoLqsaVWvdf2jQ47YFpux9iWDVhePwr", 1_110_453 * ANLOG, None),
	("anBHThJo3sMZWBF2Z7ogtfmzRf4wwiXQdKf9QHQFysWGWfyQd", 1_045_030 * ANLOG, None),
	("an8nnNEVZXHFRDW86AjdFajLwVxHjtfT85ZDmqK5HPnwsWDpM", 968_254 * ANLOG, None),
	("an8UgwNdqGCK8A1DAeZPrUr19YXCUxP6jJ7vG3EmruHAQcq8D", 912_329 * ANLOG, None),
	("an9yfZyEHuVENEsnH14ZS5FWuFiozBzbxoWZUZuhZQnF6wE4o", 903_273 * ANLOG, None),
	("an89nEZjV9sJrCB57AafrX2ACD4Ejuf3tZ2mJ8Xxu3wSdBtF2", 783_216 * ANLOG, None),
	("an9nQ45DixeQBh8E45Eh1zdCtkmpa7tyJmryQhepWuYmWURPD", 775_831 * ANLOG, None),
	("an6mSw6sr9j9J5Jaj4gxvUtbquPWKv2efaR97TGuPaALSGsYL", 743_623 * ANLOG, None),
	("an7SFZ7xSXZwBuTpbyd2SZaWkWNdnNkdra3AaVZSwku21RvcW", 594_663 * ANLOG, None),
	("an8LveLgKEcPGy893QtPzuUD2RgL1Xz2TwGyyNfeiGW86yjmu", 588_139 * ANLOG, None),
	("an8V4isY54q2G64dC3pkoYjdby5KpiFj8APDec44AmJzD4E2v", 444_092 * ANLOG, None),
	("an5dnmNtX4tLqvrLVg5wqy4JVWmsSArSq9Jrz8q5RmXHGAEG1", 425_909 * ANLOG, None),
	("an8XZK5FL61vAfeDwJUZMxpdT1iQzmn8kVT7n7uuEqefJLwzg", 373_597 * ANLOG, None),
	("anB1xyuy7Tb9hXXsqFiyWDqharGTaXAZ8g6bWsCDfBAuqEXpQ", 338_131 * ANLOG, None),
	("anANLio3NQaUtyjDo4fH1h7cZ8Kw1upUSNQWdC8a8nc1ZETsV", 338_821 * ANLOG, None),
	("an8SagSYFwxAPQcGkJHSr23e3izC8pjz3vx2ekPXJWcvfKXKQ", 313_822 * ANLOG, None),
	("an7hyq3UcJL27UgHFj3BDc2Jo2qNcn26eyHF7P4gbgvopYJdJ", 308_411 * ANLOG, None),
	("an7csmfdBWadadFMqDL34x5goCiUw7f1uHW7yjrcW84HLNcBA", 160_086 * ANLOG, None),
	("an9f4D2BoRHeRDcs8ethZVXgyiR3h3x3iYU78fdEwd3QqdG2C", 160_086 * ANLOG, None),
	("an9KxpzxnvdfoMMmUPxp1Xrif297m9xaWQaHJTYQx17DP5pPu", 160_086 * ANLOG, None),
	("an9LZjTDT7AbPDg5ZEmTmQHKVHun2hPxTkZYeFM5RbwB8b6jT", 160_086 * ANLOG, None),
	("an6tn7ySntRrvWKRaxQhDe3YRnoFThmLE9HyrnzS52Yqzmz4d", 160_086 * ANLOG, None),
	("an9f9bFSFprrcVsxkzyJ3DXRQ2cBTBr7zc7DRJ4xhtidNgUFp", 160_086 * ANLOG, None),
	("an6AoqcuFn5QR4pfUpH2ZrkrnombLNdSsJR1QweAPVhi47TAP", 160_086 * ANLOG, None),
	("anAX9u5cX5JVTJ7oVRvFQK4CpMJDBZ8UXmHzBjTvJvHdwbinh", 160_086 * ANLOG, None),
	("anALkfhvKCRoGUQo4jftTwdR2qoLbKk8R4YwgQfsSRdDvQTSp", 160_086 * ANLOG, None),
	("an8sv3uckdgzXTpTzxNfKGkYVqAkQMrxV7guuaFrjpteuqWCP", 160_086 * ANLOG, None),
	("an7qk3cBTMQwNVHUzCbgP4TCfe48LmMg9hxSJEGUda3kzqVSX", 160_086 * ANLOG, None),
	("anAsyiypD53XmqyKVW3jEVWzQrvRKwwzpGxWoPfswKA5Ws4bd", 160_086 * ANLOG, None),
	("an6S85ytt6xqvnVNvwcQpghZ8K1pyRqmDcjVbpnGdtsPiRqdo", 160_086 * ANLOG, None),
	("an6S7gKhGsY7giyAUk825BCnXbBY7udfV6rASf1erm4LqX3M6", 160_086 * ANLOG, None),
	("an8DPopdDoufFwPQHVp3sg9j2pXhsd8nHK78YLV6BP8jYmmWH", 160_086 * ANLOG, None),
	("an6assLp4Pw2wtM82tkkqy7JKFvqqWhtfWenAtT9mYYMzTxPy", 160_086 * ANLOG, None),
	("an9tCi25sDJRbVuRZ1pjcEynZXmEUiLSa6iucv5yuaB6zpYVe", 160_086 * ANLOG, None),
	("an9q3uXAjV494U8U5TEY86KZomuxSryLVTouUv4LVEz8dPPQm", 160_086 * ANLOG, None),
];
