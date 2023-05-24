use std::rc::Rc;

use ash::Entry;

pub struct EntryCtx {
    pub entry: Entry
}

impl EntryCtx {
    pub fn new() -> Rc<EntryCtx> {
        Rc::new(EntryCtx {
            entry: Entry::linked()
        })
    }
}