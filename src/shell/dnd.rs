use crate::win::{cursor_pos, desktop_at};
use std::{cell::Cell, path::PathBuf, rc::Rc};
use windows::{
    Win32::{
        Foundation::{COLORREF, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, HWND, POINT, S_OK, SIZE},
        Graphics::Gdi::{DeleteObject, HBITMAP},
        System::{
            Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree, IDataObject},
            Ole::{DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, IDropSource, IDropSource_Impl},
            SystemServices::{MK_LBUTTON, MK_RBUTTON, MODIFIERKEYS_FLAGS},
        },
        UI::Shell::*,
    },
    core::{BOOL, HRESULT, Result, implement},
};

pub fn pick_folder(h: HWND) -> Option<PathBuf> {
    unsafe {
        let d: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        d.SetOptions(d.GetOptions().ok()? | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
        d.Show(Some(h)).ok()?;
        let p = d.GetResult().ok()?.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as _));
        s.map(PathBuf::from)
    }
}

pub struct DragImage {
    pub bitmap: HBITMAP,
    pub size: (i32, i32),
    pub offset: (i32, i32),
}

fn attach(data: &IDataObject, img: DragImage) {
    let shdi = SHDRAGIMAGE {
        sizeDragImage: SIZE { cx: img.size.0, cy: img.size.1 },
        ptOffset: POINT { x: img.offset.0, y: img.offset.1 },
        hbmpDragImage: img.bitmap,
        crColorKey: COLORREF(0xFFFF_FFFF),
    };
    let helper: Result<IDragSourceHelper2> = unsafe { CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER) };
    let r = helper.and_then(|h| unsafe {
        let _ = h.SetFlags(DSH_ALLOWDROPDESCRIPTIONTEXT.0 as u32);
        h.InitializeFromBitmap(&shdi, data)
    });
    if r.is_err() {
        let _ = unsafe { DeleteObject(img.bitmap.into()) };
    }
}

#[implement(IDropSource)]
struct Source {
    desk: bool,
    landed: Rc<Cell<bool>>,
}

impl IDropSource_Impl for Source_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape.as_bool() {
            return DRAGDROP_S_CANCEL;
        }
        if keys.0 & (MK_LBUTTON.0 | MK_RBUTTON.0) != 0 {
            return S_OK;
        }
        if self.desk && desktop_at(cursor_pos()) {
            self.landed.set(true);
            return DRAGDROP_S_CANCEL;
        }
        DRAGDROP_S_DROP
    }

    fn GiveFeedback(&self, _: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

pub fn drag_out(h: HWND, paths: &[PathBuf], image: Option<DragImage>, move_only: bool, desk: bool) -> bool {
    let Some(obj) = super::data::object(paths) else { return false };
    if let Some(img) = image {
        attach(&obj, img);
    }
    let effects = if move_only { DROPEFFECT_MOVE } else { DROPEFFECT_MOVE | DROPEFFECT_COPY | DROPEFFECT_LINK };
    let landed = Rc::new(Cell::new(false));
    let source: IDropSource = Source { desk, landed: landed.clone() }.into();
    let _ = unsafe { SHDoDragDrop(Some(h), &obj, &source, effects) };
    landed.get()
}
