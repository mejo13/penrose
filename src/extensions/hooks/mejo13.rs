//! My own hooks
use crate::{
    core::{
        hooks::{EventHook, ManageHook},
        State,
    },
    x::{XConn, XEvent},
    Result, Xid,
};

// TODO
// - check behavior from clients which switched state 'tiled -> float' or vice versa
// - change name of struct
/// Changes default tiled client positioning and focus movement when unmaping
#[derive(Debug)]
pub struct ClientPositioning;
impl<X: XConn> EventHook<X> for ClientPositioning {
    fn call(&mut self, event: &XEvent, state: &mut State<X>, _: &X) -> Result<bool> {
        if let (XEvent::UnmapNotify(xid), Some(stack)) = (event, state.client_set.current_stack()) {
            if stack.head() != xid
                && state.client_set.contains(xid)
                && !state.client_set.floating.contains_key(xid)
            {
                state.client_set.swap_up();
            }
        }
        Ok(true)
    }
}
impl<X: XConn> ManageHook<X> for ClientPositioning {
    fn call(&mut self, client: Xid, state: &mut State<X>, _: &X) -> Result<()> {
        if !state.client_set.floating.contains_key(&client) {
            state.client_set.swap_down();
        }
        Ok(())
    }
}
