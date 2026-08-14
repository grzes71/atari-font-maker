use afm_core::exporters::ViewExportRegion;
use afm_core::view::{
    ViewImportOptions, ViewReplaceOptions, extract_view_import, fill_area, replace_char_x_with_y,
};

#[test]
fn test_view_replace_char_with_font_filter() {
    let mut view = vec![0u8; 40 * 26];
    let mut line_fonts = vec![1u8; 26];
    line_fonts[1] = 2; // Line 1 is on Font 2

    // Set char 10 at (5, 0) on line 0 (Font 1) and (5, 1) on line 1 (Font 2)
    view[0 * 40 + 5] = 10;
    view[1 * 40 + 5] = 10;

    // Replace 10 -> 20 only for Font 1
    let region = ViewExportRegion::new(0, 0, 40, 26);
    let options = ViewReplaceOptions {
        char_x: 10,
        char_y: 20,
        active_fonts: [true, false, false, false],
    };
    replace_char_x_with_y(&mut view, 40, 26, region, options, &line_fonts);

    assert_eq!(view[0 * 40 + 5], 20, "Font 1 line should be replaced");
    assert_eq!(view[1 * 40 + 5], 10, "Font 2 line should remain unchanged");
}

#[test]
fn test_view_fill_area() {
    let mut view = vec![0u8; 40 * 26];
    let region = ViewExportRegion::new(2, 2, 4, 3); // 4x3 box from (2, 2) to (5, 4)

    fill_area(&mut view, 40, 26, region, 0x55);

    for y in 0..26 {
        for x in 0..40 {
            let val = view[y * 40 + x];
            if (2..=5).contains(&x) && (2..=4).contains(&y) {
                assert_eq!(val, 0x55, "Inside box should be 0x55");
            } else {
                assert_eq!(val, 0x00, "Outside box should be 0x00");
            }
        }
    }
}

#[test]
fn test_extract_view_import() {
    let source = (0..100).map(|i| i as u8).collect::<Vec<u8>>(); // 10 lines of 10 bytes
    let options = ViewImportOptions {
        line_width: 10,
        skip_x: 2,
        skip_y: 1,
        copy_w: 4,
        copy_h: 3,
        target_w: 40,
        target_h: 26,
    };

    let target = extract_view_import(&source, options);

    // Row 0 of target should have source row 1 (10..19), bytes from skip_x (2..5) -> [12, 13, 14, 15]
    assert_eq!(&target[0..4], &[12, 13, 14, 15]);
    // Row 1 of target should have source row 2 (20..29), bytes from skip_x (2..5) -> [22, 23, 24, 25]
    assert_eq!(&target[40..44], &[22, 23, 24, 25]);
    // Row 2 of target should have source row 3 (30..39), bytes from skip_x (2..5) -> [32, 33, 34, 35]
    assert_eq!(&target[80..84], &[32, 33, 34, 35]);
}
