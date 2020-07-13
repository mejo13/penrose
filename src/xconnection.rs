/*! API wrapper for talking to the X server using XCB
 *
 *  The crate used by penrose for talking to the X server is rust-xcb, which
 *  is a set of bindings for the C level XCB library that are autogenerated
 *  from an XML spec. The XML files can be found
 *  [here](https://github.com/rtbo/rust-xcb/tree/master/xml) and are useful
 *  as reference for how the API works. Sections have been converted and added
 *  to the documentation of the method calls and enums present in this module.
 */
use crate::data_types::{KeyBindings, KeyCode, Region, WinId};
use crate::screen::Screen;
use std::collections::HashMap;
use xcb;

const WM_NAME: &'static str = "penrose";

/*
 * pulling out bitmasks to make the following xcb / xrandr calls easier to parse visually
 */
const NOTIFY_MASK: u16 = xcb::randr::NOTIFY_MASK_CRTC_CHANGE as u16;
const GRAB_MODE_ASYNC: u8 = xcb::GRAB_MODE_ASYNC as u8;
const INPUT_FOCUS_PARENT: u8 = xcb::INPUT_FOCUS_PARENT as u8;
const PROP_MODE_REPLACE: u8 = xcb::PROP_MODE_REPLACE as u8;
const ATOM_WINDOW: u32 = xcb::xproto::ATOM_WINDOW;
const WIN_BORDER: u16 = xcb::CONFIG_WINDOW_BORDER_WIDTH as u16;
const WIN_HEIGHT: u16 = xcb::CONFIG_WINDOW_HEIGHT as u16;
const WIN_WIDTH: u16 = xcb::CONFIG_WINDOW_WIDTH as u16;
const WIN_X: u16 = xcb::CONFIG_WINDOW_X as u16;
const WIN_Y: u16 = xcb::CONFIG_WINDOW_Y as u16;
const NEW_WINDOW_MASK: &[(u32, u32)] = &[(
    xcb::CW_EVENT_MASK,
    xcb::EVENT_MASK_ENTER_WINDOW | xcb::EVENT_MASK_LEAVE_WINDOW,
)];
const MOUSE_MASK: u16 = (xcb::EVENT_MASK_BUTTON_PRESS
    | xcb::EVENT_MASK_BUTTON_RELEASE
    | xcb::EVENT_MASK_POINTER_MOTION) as u16;
const EVENT_MASK: &[(u32, u32)] = &[(
    xcb::CW_EVENT_MASK,
    xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY as u32,
)];

// TODO: this list has been copied from atoms used in other WMs, not using everything
//       yet so work out which ones we need to keep and which we can drop.
const ATOMS: &[&'static str] = &[
    "MANAGER",
    "UTF8_STRING",
    "WM_CLASS",
    "WM_DELETE_WINDOW",
    "WM_PROTOCOLS",
    "WM_STATE",
    "WM_TAKE_FOCUS",
    "_NET_ACTIVE_WINDOW",
    "_NET_CLIENT_LIST",
    "_NET_SUPPORTED",
    "_NET_SUPPORTING_WM_CHECK",
    "_NET_SYSTEM_TRAY_OPCODE",
    "_NET_SYSTEM_TRAY_ORIENTATION",
    "_NET_SYSTEM_TRAY_ORIENTATION_HORZ",
    "_NET_SYSTEM_TRAY_S0",
    "_NET_WM_NAME",
    "_NET_WM_STATE",
    "_NET_WM_STATE_FULLSCREEN",
    "_NET_WM_WINDOW_TYPE",
    "_NET_WM_WINDOW_TYPE_DIALOG",
    "_XEMBED",
    "_XEMBED_INFO",
];

/**
 * Wrapper around the low level XCB event types that require casting to work with.
 * Not all event fields are extracted so check the XCB documentation and update
 * accordingly if you need access to something that isn't currently passed through
 * to the WindowManager event loop.
 *
 * https://github.com/rtbo/rust-xcb/xml/xproto.xml
 *
 * ### XCB Level events
 *
 * *MapNotify* - a window was mapped
 *   - _event_ (WinId):
 *     The window which was mapped or its parent, depending on
 *     whether `StructureNotify` or `SubstructureNotify` was selected.
 *   - _window_ (WinId):
 *     The window that was mapped.
 *   - _override_redirect_ (bool):
 *     We should ignore this window if true
 *
 * *UnmapNotify* - a window was unmapped
 *   - _event_ (WinId):
 *     The window which was unmapped or its parent, depending on
 *     whether `StructureNotify` or `SubstructureNotify` was selected.
 *   - _window_ (WinId):
 *     The window that was unmapped.
 *   - _from-configure_ (bool):
 *     - 'true' if the event was generated as a result of a resizing of
 *       the window's parent when `window` had a win_gravity of `UnmapGravity`.
 *
 * *EnterNotify* - the pointer is now in a different window
 *   - _event_ (WinId):
 *     The window on which the event was generated.
 *   - _child_ (WinId):
 *     If the window has sub-windows then this is the ID of the window
 *     that the pointer ended on, XCB_WINDOW_NONE otherwise.
 *   - _root_ (WinId):
 *     The root window for the final cursor position.
 *   - _root-x, root-y_ (i16, i16):
 *     The coordinates of the pointer relative to 'root's origin.
 *   - _event-x, event-y_ (i16, i16):
 *     The coordinates of the pointer relative to the event window's origin.
 *   - _mode_ (NotifyMode enum)
 *     - Normal, Grab, Ungrab, WhileGrabbed
 *
 * *LeaveNotify* - the pointer has left a window
 *   - Same fields as *EnterNotify*
 *
 * *DestroyNotify* - a window has been destroyed
 *   - _event_ (WinId):
 *     The reconfigured window or its parent, depending on whether
 *     `StructureNotify` or `SubstructureNotify` was selected.
 *   - _window_ (WinId):
 *     The window that was destroyed.
 *
 * *KeyPress* - a keyboard key was pressed / released
 *   - _detail_ (u8):
 *     Keycode of the key that was pressed
 *   - _event_ (u16):
 *     The modifier masks being held when the key was pressed
 *   - _child_ (WinId):
 *     If the window has sub-windows then this is the ID of the window
 *     that the pointer ended on, XCB_WINDOW_NONE otherwise.
 *   - _root_ (WinId):
 *     The root window for the final cursor position.
 *   - _root-x, root-y_ (i16, i16):
 *     The coordinates of the pointer relative to 'root's origin.
 *   - _event-x, event-y_ (i16, i16):
 *     The coordinates of the pointer relative to the event window's origin.
 *
 * *ButtonPress* - a mouse button was pressed
 *   - _detail_ (u8):
 *     The button that was pressed
 *   - _event_ (u16):
 *     The modifier masks being held when the button was pressed
 *   - _child_ (WinId):
 *     If the window has sub-windows then this is the ID of the window
 *     that the pointer ended on, XCB_WINDOW_NONE otherwise.
 *   - _root_ (WinId):
 *     The root window for the final cursor position.
 *   - _root-x, root-y_ (i16, i16):
 *     The coordinates of the pointer relative to 'root's origin.
 *   - _event-x, event-y_ (i16, i16):
 *     The coordinates of the pointer relative to the event window's origin.
 *
 * *ButtonRelease* - a mouse button was released
 *   - same fields as *ButtonPress*
 */
#[derive(Debug, Copy, Clone)]
pub enum XEvent {
    /// xcb docs: https://www.mankier.com/3/xcb_input_raw_button_press_event_t
    ButtonPress,

    /// xcb docs: https://www.mankier.com/3/xcb_input_raw_button_press_event_t
    ButtonRelease,

    /// xcb docs: https://www.mankier.com/3/xcb_input_device_key_press_event_t
    KeyPress { code: KeyCode },

    /// MapNotifyEvent
    /// xcb docs: https://www.mankier.com/3/xcb_xkb_map_notify_event_t
    Map { window: WinId, ignore: bool },

    /// xcb docs: https://www.mankier.com/3/xcb_enter_notify_event_t
    Enter { window: WinId },

    /// xcb docs: https://www.mankier.com/3/xcb_enter_notify_event_t
    Leave { window: WinId },

    /// MapNotifyEvent
    /// xcb docs: https://www.mankier.com/3/xcb_destroy_notify_event_t
    Destroy { window: WinId },
}

/// A handle on a running X11 connection that we can use for issuing X requests
pub trait XConn {
    /// Flush pending actions to the X event loop
    fn flush(&self) -> bool;

    /// Wait for the next event from the X server and return it as an XEvent
    fn wait_for_event(&self) -> Option<XEvent>;

    /// Determine the currently connected CRTCs and return their details
    fn current_outputs(&self) -> Vec<Screen>;

    /// Reposition the window identified by 'id' to the specifed region
    fn position_window(&self, id: WinId, r: Region, border: u32);

    /// Mark the given window as newly created
    fn mark_new_window(&self, id: WinId);

    /// Map a window to the display. Called each time a map_notify event is received
    fn map_window(&self, id: WinId);

    /// Unmap a window from the display. Called each time an unmap_notify event is received
    fn unmap_window(&self, id: WinId);

    /// Send an X event to the target window
    fn send_client_event(&self, id: WinId, atom_name: &str);

    /// Mark the given client as having focus
    fn focus_client(&self, id: WinId);

    /// Change the border color for the given client
    fn set_client_border_color(&self, id: WinId, color: u32);

    /**
     * Notify the X server that we are intercepting the user specified key bindings
     * and prevent them being passed through to the underlying applications. This
     * is what determines which key press events end up being sent through in the
     * main event loop for the WindowManager.
     */
    fn grab_keys(&self, key_bindings: &KeyBindings);

    /// Set required Net* X properties to ensure compatability
    fn set_wm_properties(&self);

    /**
     * Use the xcb api to query a string property for a window by window ID and poperty name.
     * Can fail if the property name is invalid or we get a malformed response from xcb.
     */
    fn str_prop(&self, id: u32, name: &str) -> Result<String, String>;

    /// Fetch an atom prop by name for a particular window ID
    fn atom_prop(&self, id: u32, name: &str) -> Result<u32, String>;
}

/// Handles communication with an X server via xcb
pub struct XcbConnection {
    conn: xcb::Connection,
    atoms: HashMap<&'static str, u32>,
}

impl XcbConnection {
    /// Establish a new connection to the running X server. Fails if unable to connect
    pub fn new() -> XcbConnection {
        let (conn, _) = match xcb::Connection::connect(None) {
            Err(e) => panic!("unable to establish connection to X server: {}", e),
            Ok(conn) => conn,
        };

        let mut atoms = HashMap::with_capacity(ATOMS.len());
        for atom in ATOMS.iter() {
            // https://www.mankier.com/3/xcb_intern_atom
            let val = xcb::intern_atom(
                &conn, // xcb connection to X11
                false, // return the atom ID even if it doesn't already exists
                atom,  // name of the atom to retrieve
            )
            .get_reply()
            .expect(&format!("unable to intern xcb atom '{}'", atom))
            .atom();

            atoms.insert(*atom, val);
        }

        XcbConnection { conn, atoms }
    }

    fn atom(&self, name: &str) -> u32 {
        *self
            .atoms
            .get(name)
            .expect(&format!("{} is not a known atom", name))
    }

    fn new_stub_window(&self) -> WinId {
        let screen = match self.conn.get_setup().roots().nth(0) {
            None => panic!("unable to get handle for screen"),
            Some(s) => s,
        };

        let win_id = self.conn.generate_id();
        let root = screen.root();

        // xcb docs: https://www.mankier.com/3/xcb_create_window
        xcb::create_window(
            &self.conn, // xcb connection to X11
            0,          // new window's depth
            win_id,     // ID to be used for referring to the window
            root,       // parent window
            0,          // x-coordinate
            0,          // y-coordinate
            1,          // width
            1,          // height
            0,          // border width
            0,          // class (i _think_ 0 == COPY_FROM_PARENT?)
            0,          // visual (i _think_ 0 == COPY_FROM_PARENT?)
            &[],        // value list? (value mask? not documented either way...)
        );

        win_id
    }
}

impl XConn for XcbConnection {
    fn flush(&self) -> bool {
        self.conn.flush()
    }

    fn wait_for_event(&self) -> Option<XEvent> {
        self.conn.wait_for_event().and_then(|event| {
            let etype = event.response_type();
            match etype {
                xcb::BUTTON_PRESS => None,

                xcb::BUTTON_RELEASE => None,

                xcb::KEY_PRESS => {
                    let e: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&event) };
                    Some(XEvent::KeyPress {
                        code: KeyCode::from_key_press(e),
                    })
                }

                xcb::MAP_NOTIFY => {
                    let e: &xcb::MapNotifyEvent = unsafe { xcb::cast_event(&event) };
                    Some(XEvent::Map {
                        window: e.window(),
                        ignore: e.override_redirect(),
                    })
                }

                xcb::ENTER_NOTIFY => {
                    let e: &xcb::EnterNotifyEvent = unsafe { xcb::cast_event(&event) };
                    Some(XEvent::Enter { window: e.event() })
                }

                xcb::LEAVE_NOTIFY => {
                    let e: &xcb::LeaveNotifyEvent = unsafe { xcb::cast_event(&event) };
                    Some(XEvent::Leave { window: e.event() })
                }

                xcb::DESTROY_NOTIFY => {
                    let e: &xcb::MapNotifyEvent = unsafe { xcb::cast_event(&event) };
                    Some(XEvent::Destroy { window: e.window() })
                }

                // NOTE: ignoring other event types
                _ => None,
            }
        })
    }

    fn current_outputs(&self) -> Vec<Screen> {
        let win_id = self.new_stub_window();

        // xcb docs: https://www.mankier.com/3/xcb_randr_get_screen_resources
        let resources = xcb::randr::get_screen_resources(&self.conn, win_id);

        // xcb docs: https://www.mankier.com/3/xcb_randr_get_crtc_info
        match resources.get_reply() {
            Err(e) => panic!("error reading X screen resources: {}", e),
            Ok(reply) => reply
                .crtcs()
                .iter()
                .flat_map(|c| xcb::randr::get_crtc_info(&self.conn, *c, 0).get_reply())
                .enumerate()
                .map(|(i, r)| Screen::from_crtc_info_reply(r, i))
                .filter(|s| s.true_region.width() > 0)
                .collect(),
        }
    }

    fn position_window(&self, id: WinId, r: Region, border: u32) {
        let (x, y, w, h) = r.values();
        xcb::configure_window(
            &self.conn,
            id,
            &[
                (WIN_X, x),
                (WIN_Y, y),
                (WIN_WIDTH, w),
                (WIN_HEIGHT, h),
                (WIN_BORDER, border),
            ],
        );
    }

    fn mark_new_window(&self, id: WinId) {
        // xcb docs: https://www.mankier.com/3/xcb_change_window_attributes
        xcb::change_window_attributes(&self.conn, id, NEW_WINDOW_MASK);
    }

    fn map_window(&self, id: WinId) {
        xcb::map_window(&self.conn, id);
    }

    fn unmap_window(&self, id: WinId) {
        xcb::unmap_window(&self.conn, id);
    }

    fn send_client_event(&self, id: WinId, atom_name: &str) {
        let atom = self.atom(atom_name);
        let wm_protocols = self.atom("WM_PROTOCOLS");
        let data = xcb::ClientMessageData::from_data32([atom, xcb::CURRENT_TIME, 0, 0, 0]);
        let event = xcb::ClientMessageEvent::new(32, id, wm_protocols, data);
        xcb::send_event(&self.conn, false, id, xcb::EVENT_MASK_NO_EVENT, &event);
    }

    fn focus_client(&self, id: WinId) {
        let root = match self.conn.get_setup().roots().nth(0) {
            None => panic!("unable to get handle for screen"),
            Some(screen) => screen.root(),
        };

        let prop = self.atom("_NET_ACTIVE_WINDOW");

        // xcb docs: https://www.mankier.com/3/xcb_set_input_focus
        xcb::set_input_focus(
            &self.conn,         // xcb connection to X11
            INPUT_FOCUS_PARENT, // focus the parent when focus is lost
            id,                 // window to focus
            0,                  // current time to avoid network race conditions (0 == current time)
        );

        // xcb docs: https://www.mankier.com/3/xcb_change_property
        xcb::change_property(
            &self.conn,        // xcb connection to X11
            PROP_MODE_REPLACE, // discard current prop and replace
            root,              // window to change prop on
            prop,              // prop to change
            ATOM_WINDOW,       // type of prop
            32,                // data format (8/16/32-bit)
            &[id],             // data
        );
    }

    fn set_client_border_color(&self, id: WinId, color: u32) {
        xcb::change_window_attributes(&self.conn, id, &[(xcb::CW_BORDER_PIXEL, color)]);
    }

    fn grab_keys(&self, key_bindings: &KeyBindings) {
        let screen = self.conn.get_setup().roots().nth(0).unwrap();
        let root = screen.root();

        // xcb docs: https://www.mankier.com/3/xcb_randr_select_input
        let input = xcb::randr::select_input(&self.conn, root, NOTIFY_MASK);
        match input.request_check() {
            Err(e) => panic!("randr error: {}", e),
            Ok(_) => {
                for k in key_bindings.keys() {
                    // xcb docs: https://www.mankier.com/3/xcb_grab_key
                    xcb::grab_key(
                        &self.conn,      // xcb connection to X11
                        false,           // don't pass grabbed events through to the client
                        root,            // the window to grab: in this case the root window
                        k.mask,          // modifiers to grab
                        k.code,          // keycode to grab
                        GRAB_MODE_ASYNC, // don't lock pointer input while grabbing
                        GRAB_MODE_ASYNC, // don't lock keyboard input while grabbing
                    );
                }
            }
        }

        // TODO: this needs to be more configurable by the user
        for mouse_button in &[1, 3] {
            // xcb docs: https://www.mankier.com/3/xcb_grab_button
            xcb::grab_button(
                &self.conn,             // xcb connection to X11
                false,                  // don't pass grabbed events through to the client
                root,                   // the window to grab: in this case the root window
                MOUSE_MASK,             // which events are reported to the client
                GRAB_MODE_ASYNC,        // don't lock pointer input while grabbing
                GRAB_MODE_ASYNC,        // don't lock keyboard input while grabbing
                xcb::NONE,              // don't confine the cursor to a specific window
                xcb::NONE,              // don't change the cursor type
                *mouse_button,          // the button to grab
                xcb::MOD_MASK_4 as u16, // modifiers to grab
            );
        }

        // xcb docs: https://www.mankier.com/3/xcb_change_window_attributes
        xcb::change_window_attributes(&self.conn, root, EVENT_MASK);
        &self.conn.flush();
    }

    fn set_wm_properties(&self) {
        let screen = match self.conn.get_setup().roots().nth(0) {
            None => panic!("unable to get handle for screen"),
            Some(s) => s,
        };
        let root = screen.root();

        // NOTE: The following uses xcb_change_property under the hood
        //       xcb docs: https://www.mankier.com/3/xcb_change_property
        let check_win = self.new_stub_window();

        xcb::change_property(
            &self.conn,                            // xcb connection to X11
            PROP_MODE_REPLACE,                     // discard current prop and replace
            check_win,                             // window to change prop on
            self.atom("_NET_SUPPORTING_WM_CHECK"), // prop to change
            ATOM_WINDOW,                           // type of prop
            32,                                    // data format (8/16/32-bit)
            &[check_win],                          // data
        );
        xcb::change_property(
            &self.conn,                // xcb connection to X11
            PROP_MODE_REPLACE,         // discard current prop and replace
            check_win,                 // window to change prop on
            self.atom("_NET_WM_NAME"), // prop to change
            xcb::xproto::ATOM_STRING,  // type of prop
            8,                         // data format (8/16/32-bit)
            &[WM_NAME],                // data
        );
        xcb::change_property(
            &self.conn,                            // xcb connection to X11
            PROP_MODE_REPLACE,                     // discard current prop and replace
            root,                                  // window to change prop on
            self.atom("_NET_SUPPORTING_WM_CHECK"), // prop to change
            ATOM_WINDOW,                           // type of prop
            32,                                    // data format (8/16/32-bit)
            &[check_win],                          // data
        );

        // EWMH support
        let supported: Vec<u32> = ATOMS.iter().map(|a| self.atom(a)).collect();
        xcb::change_property(
            &self.conn,                  // xcb connection to X11
            PROP_MODE_REPLACE,           // discard current prop and replace
            root,                        // window to change prop on
            self.atom("_NET_SUPPORTED"), // prop to change
            xcb::xproto::ATOM_ATOM,      // type of prop
            32,                          // data format (8/16/32-bit)
            &supported,                  // data
        );
        xcb::delete_property(&self.conn, root, self.atom("_NET_CLIENT_LIST"));
    }

    fn str_prop(&self, id: u32, name: &str) -> Result<String, String> {
        // xcb docs: https://www.mankier.com/3/xcb_get_property
        let cookie = xcb::get_property(
            &self.conn,      // xcb connection to X11
            false,           // should the property be deleted
            id,              // target window to query
            self.atom(name), // the property we want
            xcb::ATOM_ANY,   // the type of the property
            0,               // offset in the property to retrieve data from
            1024,            // how many 32bit multiples of data to retrieve
        );

        match cookie.get_reply() {
            Err(e) => Err(format!("unable to fetch window property: {}", e)),
            Ok(reply) => match String::from_utf8(reply.value().to_vec()) {
                Err(e) => Err(format!("invalid utf8 resonse from xcb: {}", e)),
                Ok(s) => Ok(s),
            },
        }
    }

    fn atom_prop(&self, id: u32, name: &str) -> Result<u32, String> {
        // xcb docs: https://www.mankier.com/3/xcb_get_property
        let cookie = xcb::get_property(
            &self.conn,      // xcb connection to X11
            false,           // should the property be deleted
            id,              // target window to query
            self.atom(name), // the property we want
            xcb::ATOM_ANY,   // the type of the property
            0,               // offset in the property to retrieve data from
            1024,            // how many 32bit multiples of data to retrieve
        );

        match cookie.get_reply() {
            Err(e) => Err(format!("unable to fetch window property: {}", e)),
            Ok(reply) => {
                if reply.value_len() <= 0 {
                    Err(format!("property '{}' was empty for id: {}", name, id))
                } else {
                    Ok(reply.value()[0])
                }
            }
        }
    }
}

pub struct MockXConn {
    screens: Vec<Screen>,
}

impl MockXConn {
    pub fn new(screens: Vec<Screen>) -> Self {
        MockXConn { screens }
    }
}

impl XConn for MockXConn {
    fn flush(&self) -> bool {
        true
    }
    fn wait_for_event(&self) -> Option<XEvent> {
        None
    }
    fn current_outputs(&self) -> Vec<Screen> {
        self.screens.clone()
    }
    fn position_window(&self, _: WinId, _: Region, _: u32) {}
    fn mark_new_window(&self, _: WinId) {}
    fn map_window(&self, _: WinId) {}
    fn unmap_window(&self, _: WinId) {}
    fn send_client_event(&self, _: WinId, _: &str) {}
    fn focus_client(&self, _: WinId) {}
    fn set_client_border_color(&self, _: WinId, _: u32) {}
    fn grab_keys(&self, _: &KeyBindings) {}
    fn set_wm_properties(&self) {}
    fn str_prop(&self, _: u32, name: &str) -> Result<String, String> {
        Ok(String::from(name))
    }
    fn atom_prop(&self, id: u32, _: &str) -> Result<u32, String> {
        Ok(id)
    }
}