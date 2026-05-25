// ---------------------------------------------------------------------------
// Shape Sample
// ---------------------------------------------------------------------------

pub(super) const SKELETON_CANDLE_COUNT: usize = 64;
pub(super) const MIN_CANDLE_SPACING: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct SkeletonCandle {
    pub(super) open: f32,
    pub(super) high: f32,
    pub(super) low: f32,
    pub(super) close: f32,
    pub(super) volume: f32,
}

impl SkeletonCandle {
    const fn new(open: f32, high: f32, low: f32, close: f32, volume: f32) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
        }
    }
}

// Shape-only data normalized from a real BTC 1h Hyperliquid candleSnapshot.
pub(super) const API_SAMPLE_CANDLES: [SkeletonCandle; 64] = [
    SkeletonCandle::new(0.3670, 0.5056, 0.3114, 0.4671, 0.4684),
    SkeletonCandle::new(0.4671, 0.5310, 0.3885, 0.4553, 0.4327),
    SkeletonCandle::new(0.4553, 0.4944, 0.3885, 0.4192, 0.2574),
    SkeletonCandle::new(0.4192, 0.6218, 0.3943, 0.4871, 0.5044),
    SkeletonCandle::new(0.4871, 0.4939, 0.2484, 0.2489, 0.6007),
    SkeletonCandle::new(0.2489, 0.4143, 0.1552, 0.2289, 0.4659),
    SkeletonCandle::new(0.2289, 0.3494, 0.2089, 0.3080, 0.2692),
    SkeletonCandle::new(0.3084, 0.3914, 0.2689, 0.3807, 0.3944),
    SkeletonCandle::new(0.3807, 0.4285, 0.2416, 0.3714, 0.2962),
    SkeletonCandle::new(0.3719, 0.4685, 0.3592, 0.4290, 0.2225),
    SkeletonCandle::new(0.4295, 0.5559, 0.3997, 0.5242, 0.3286),
    SkeletonCandle::new(0.5242, 0.5691, 0.3982, 0.4368, 0.3833),
    SkeletonCandle::new(0.4368, 0.4480, 0.2655, 0.3226, 0.4907),
    SkeletonCandle::new(0.3221, 0.4685, 0.2704, 0.4353, 0.4559),
    SkeletonCandle::new(0.4353, 0.4568, 0.2304, 0.2777, 0.4372),
    SkeletonCandle::new(0.2777, 0.4095, 0.2421, 0.3860, 0.3003),
    SkeletonCandle::new(0.3865, 0.4456, 0.2143, 0.3519, 0.6571),
    SkeletonCandle::new(0.3519, 0.4192, 0.0000, 0.1230, 0.7945),
    SkeletonCandle::new(0.1235, 0.2616, 0.0937, 0.1776, 0.5052),
    SkeletonCandle::new(0.1781, 0.3812, 0.1274, 0.3641, 0.5301),
    SkeletonCandle::new(0.3646, 0.4251, 0.2967, 0.3426, 0.4457),
    SkeletonCandle::new(0.3426, 0.3904, 0.2879, 0.3182, 0.3788),
    SkeletonCandle::new(0.3187, 0.4539, 0.2997, 0.3255, 0.3199),
    SkeletonCandle::new(0.3255, 0.4344, 0.3255, 0.4241, 0.3165),
    SkeletonCandle::new(0.4246, 0.4353, 0.3992, 0.4056, 0.2717),
    SkeletonCandle::new(0.4061, 0.4061, 0.2377, 0.2753, 0.4787),
    SkeletonCandle::new(0.2753, 0.4046, 0.2533, 0.3324, 0.2747),
    SkeletonCandle::new(0.3328, 0.3651, 0.1781, 0.3099, 0.4228),
    SkeletonCandle::new(0.3104, 0.3714, 0.1855, 0.3216, 0.4413),
    SkeletonCandle::new(0.3221, 0.3685, 0.2172, 0.2596, 0.3006),
    SkeletonCandle::new(0.2601, 0.3309, 0.2191, 0.2767, 0.2703),
    SkeletonCandle::new(0.2772, 0.3294, 0.2626, 0.3045, 0.4399),
    SkeletonCandle::new(0.3050, 0.5730, 0.3050, 0.5115, 0.4767),
    SkeletonCandle::new(0.5115, 0.6179, 0.5076, 0.6144, 0.3555),
    SkeletonCandle::new(0.6149, 0.6423, 0.5281, 0.5481, 0.2790),
    SkeletonCandle::new(0.5486, 0.7277, 0.5144, 0.6408, 0.4587),
    SkeletonCandle::new(0.6408, 0.6808, 0.5930, 0.6496, 0.3123),
    SkeletonCandle::new(0.6496, 0.7374, 0.6491, 0.6652, 0.3106),
    SkeletonCandle::new(0.6657, 0.6916, 0.5671, 0.6022, 0.3153),
    SkeletonCandle::new(0.6022, 0.7794, 0.5905, 0.7189, 0.4833),
    SkeletonCandle::new(0.7189, 0.7374, 0.3943, 0.5173, 0.6661),
    SkeletonCandle::new(0.5178, 0.6940, 0.3543, 0.6457, 1.0000),
    SkeletonCandle::new(0.6442, 0.8365, 0.5735, 0.6413, 0.8970),
    SkeletonCandle::new(0.6413, 0.6925, 0.4334, 0.5857, 0.5142),
    SkeletonCandle::new(0.5857, 0.7706, 0.5354, 0.6979, 0.4651),
    SkeletonCandle::new(0.6984, 0.7321, 0.6101, 0.6208, 0.3340),
    SkeletonCandle::new(0.6213, 0.7716, 0.5876, 0.7423, 0.3833),
    SkeletonCandle::new(0.7423, 0.8009, 0.6486, 0.7716, 0.6801),
    SkeletonCandle::new(0.7721, 0.8019, 0.6657, 0.6657, 0.3557),
    SkeletonCandle::new(0.6657, 0.6657, 0.5271, 0.6633, 0.5064),
    SkeletonCandle::new(0.6628, 0.6906, 0.6052, 0.6872, 0.2847),
    SkeletonCandle::new(0.6872, 0.9419, 0.6711, 0.8931, 0.6117),
    SkeletonCandle::new(0.8931, 1.0000, 0.8106, 0.8404, 0.6040),
    SkeletonCandle::new(0.8399, 0.9614, 0.8219, 0.8853, 0.3800),
    SkeletonCandle::new(0.8853, 0.9566, 0.8555, 0.9517, 0.3943),
    SkeletonCandle::new(0.9512, 0.9922, 0.8838, 0.9200, 0.5081),
    SkeletonCandle::new(0.9200, 0.9722, 0.7687, 0.8180, 0.4308),
    SkeletonCandle::new(0.8175, 0.8424, 0.6613, 0.7125, 0.4997),
    SkeletonCandle::new(0.7125, 0.8536, 0.6593, 0.8433, 0.3939),
    SkeletonCandle::new(0.8433, 0.9917, 0.8429, 0.8604, 0.5842),
    SkeletonCandle::new(0.8599, 0.8907, 0.7213, 0.7218, 0.5668),
    SkeletonCandle::new(0.7213, 0.7438, 0.4929, 0.5041, 0.7446),
    SkeletonCandle::new(0.5041, 0.5959, 0.4788, 0.4988, 0.5069),
    SkeletonCandle::new(0.4993, 0.5939, 0.4441, 0.5544, 0.4585),
];
