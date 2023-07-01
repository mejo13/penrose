//! Layout behaviour that is more specialised or complex than the builtin layouts.
use crate::{
    core::layout::{Layout, Message},
    pure::{geometry::Rect, Stack},
    Xid,
};

/// Inspired by the Tatami layout available for dwm:
///   https://dwm.suckless.org/patches/tatami/
///
/// This is very much a "for looks" layout rather than something particularly practical. It
/// provides a set of 6 specific layouts depending on the number of clients present on the
/// workspace being laid out, if there are more than 6 clients then the remaining clients are
/// hidden.
///
/// ```text
/// .............................   .............................   .............................
/// .                           .   .             .             .   .             .             .
/// .                           .   .             .             .   .             .             .
/// .                           .   .             .             .   .             .             .
/// .                           .   .             .             .   .             .      2      .
/// .                           .   .             .             .   .             .             .
/// .             1             .   .      1      .     2       .   .      1      ...............
/// .                           .   .             .             .   .             .             .
/// .                           .   .             .             .   .             .             .
/// .                           .   .             .             .   .             .      3      .
/// .                           .   .             .             .   .             .             .
/// .                           .   .             .             .   .             .             .
/// .............................   .............................   .............................
///
/// .............................   .............................   .............................
/// .             .      .      .   .             .             .   .             .    .        .
/// .             .      .      .   .             .      2      .   .             .    .   3    .
/// .             .   2  .   3  .   .             ...............   .             .    ..........
/// .             .      .      .   .             .      .      .   .             .  2 .    .   .
/// .             .      .      .   .             .      .      .   .             .    .    .   .
/// .      1      ...............   .      1      .  3   .  4   .   .       1     .    . 4  .   .
/// .             .             .   .             .      .      .   .             .    .    .   .
/// .             .             .   .             .      .      .   .             .    .    . 5 .
/// .             .      4      .   .             ...............   .             ...........   .
/// .             .             .   .             .             .   .             .         .   .
/// .             .             .   .             .      5      .   .             .    6    .   .
/// .............................   .............................   .............................
/// ```
#[derive(Debug, Default, Copy, Clone)]
pub struct Tatami;

impl Tatami {
    /// Create a new [Tatami] layout returned as a trait object ready to be added to your [LayoutStack].
    pub fn boxed() -> Box<dyn Layout> {
        Box::new(Tatami)
    }
}

impl Layout for Tatami {
    fn name(&self) -> String {
        "|+|".to_string()
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Box::new(*self)
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        let apply = |rs: &[Rect]| s.iter().zip(rs).map(|(&id, &r)| (id, r)).collect();

        // We only position the first 6 clients (after that we're out of patterns)
        let n = std::cmp::min(s.len(), 6);
        let positions = match n {
            0 => vec![],

            1 => apply(&[r]),

            2 => {
                let (r1, r2) = r.split_at_mid_width();
                apply(&[r1, r2])
            }

            3 => {
                let (r1, r2) = r.split_at_mid_width();
                let (r2, r3) = r2.split_at_mid_height();
                apply(&[r1, r2, r3])
            }

            4 => {
                let (r1, r2) = r.split_at_mid_width();
                let (r2, r4) = r2.split_at_mid_height();
                let (r2, r3) = r2.split_at_mid_width();
                apply(&[r1, r2, r3, r4])
            }

            5 => {
                let (r1, r2) = r.split_at_mid_width();
                let rows = r2.as_rows(4);
                let (r2, mut rmid, r5) = (rows[0], rows[1], rows[3]);
                rmid.h += rows[2].h;
                let (r3, r4) = rmid.split_at_mid_width();
                apply(&[r1, r2, r3, r4, r5])
            }

            6 => {
                let (r1, r2) = r.split_at_mid_width();
                let cols = r2.as_columns(3);
                let h = r1.h / 4;
                let (mut r2, mut r4, mut r5) = (cols[0], cols[1], cols[2]);
                r2.h -= h;
                (r4.h, r4.y) = (r4.h - 2 * h, r4.y + h);
                (r5.h, r5.y) = (r5.h - h, r5.y + h);
                let r3 = Rect::new(r2.x + r2.w, r2.y, r2.w * 2, h);
                let r6 = Rect::new(r2.x, r2.y + r2.h, r2.w * 2, h);

                apply(&[r1, r2, r3, r4, r5, r6])
            }

            // We've capped n at 6 clients above so all cases are handled
            _ => unreachable!(),
        };

        (None, positions)
    }

    fn handle_message(&mut self, _: &Message) -> Option<Box<dyn Layout>> {
        // TODO: handle some of the standard messages that make sense for this layout
        None
    }
}