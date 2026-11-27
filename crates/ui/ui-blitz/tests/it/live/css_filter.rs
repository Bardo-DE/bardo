//! CSS `filter` painted by `vello_gpu`, the hybrid backend: each swatch must read as filtered, as the
//! CPU cases in `css_filter` have it. Needs a real GPU adapter and fails, saying so, where none opens.

use crate::css_filter::assert_rendered;
use ui_harness::Backend;

macro_rules! filter_case {
    ($($test:ident: $backend:ident, $index:expr;)*) => {
        $(
            #[test]
            fn $test() {
                assert_rendered(Backend::$backend, $index);
            }
        )*
    };
}

filter_case! {
    vello_gpu_blurs: Hybrid, 0;
    vello_gpu_brightens: Hybrid, 1;
    vello_gpu_contrasts: Hybrid, 2;
    vello_gpu_casts_a_drop_shadow: Hybrid, 3;
    vello_gpu_grays: Hybrid, 4;
    vello_gpu_rotates_the_hue: Hybrid, 5;
    vello_gpu_inverts: Hybrid, 6;
    vello_gpu_fades: Hybrid, 7;
    vello_gpu_saturates: Hybrid, 8;
    vello_gpu_sepias: Hybrid, 9;
    vello_gpu_lists_in_order: Hybrid, 10;
    vello_gpu_lists_a_colour_and_a_blur: Hybrid, 11;
    vello_gpu_lists_a_blur_and_a_colour: Hybrid, 12;
}
