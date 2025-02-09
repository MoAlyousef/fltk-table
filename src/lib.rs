#![doc = include_str!("../README.md")]
#![allow(clippy::needless_doctest_main)]

use fltk::{
    app, draw,
    enums::*,
    input,
    prelude::{GroupExt, InputExt, TableExt, WidgetBase, WidgetExt},
    table, window,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
pub struct Cell {
    label: String,
    color: Option<Color>,
    font: Option<Font>,
    font_color: Option<Color>,
    font_size: Option<i32>,
    selection_color: Option<Color>,
    align: Option<Align>,
    border_color: Option<Color>,
}

impl Cell {
    fn with_label(l: &str) -> Cell {
        Cell {
            label: l.to_string(),
            ..Default::default()
        }
    }
}

type CellMatrix = Vec<Vec<Cell>>;

// Needed to store cell information during the draw_cell call
#[derive(Default)]
struct CellData {
    pub row: i32, // row
    pub col: i32, // column
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl CellData {
    fn select(&mut self, row: i32, col: i32, x: i32, y: i32, w: i32, h: i32) {
        self.row = row;
        self.col = col;
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
    }
}

/// Contains the parameters for our table, including rows, columns and other styling params
#[derive(Debug, Clone, Copy)]
pub struct TableOpts {
    pub rows: i32,
    pub cols: i32,
    pub editable: bool,
    pub cell_color: Color,
    pub cell_font: Font,
    pub cell_font_color: Color,
    pub cell_font_size: i32,
    pub cell_selection_color: Color,
    pub cell_align: Align,
    pub cell_border_color: Color,
    pub cell_padding: i32,
    pub header_font: Font,
    pub header_frame: FrameType,
    pub header_color: Color,
    pub header_font_color: Color,
    pub header_font_size: i32,
    pub header_align: Align,
}

impl Default for TableOpts {
    fn default() -> Self {
        Self {
            rows: 1,
            cols: 1,
            editable: false,
            cell_color: Color::BackGround2,
            cell_font: Font::Helvetica,
            cell_font_color: Color::Gray0,
            cell_font_size: 14,
            cell_selection_color: Color::from_u32(0x00D3_D3D3),
            cell_align: Align::Center,
            cell_border_color: Color::Gray0,
            cell_padding: 1,
            header_font: Font::Helvetica,
            header_frame: FrameType::ThinUpBox,
            header_color: Color::FrameDefault,
            header_font_color: Color::Black,
            header_font_size: 14,
            header_align: Align::Center,
        }
    }
}

/// Smart table widget
#[derive(Clone)]
pub struct SmartTable {
    table: table::TableRow,
    inp: Option<input::Input>,
    data: Arc<Mutex<CellMatrix>>,
    row_headers: Arc<Mutex<Vec<String>>>,
    col_headers: Arc<Mutex<Vec<String>>>,
    on_update_callback: Arc<Mutex<Box<dyn FnMut(i32, i32, String) + Send>>>,
}

impl Default for SmartTable {
    fn default() -> Self {
        Self::new(0, 0, 0, 0, None)
    }
}

impl SmartTable {
    /// Construct a new SmartTable widget using coords, size and label
    pub fn new<S: Into<Option<&'static str>>>(x: i32, y: i32, w: i32, h: i32, label: S) -> Self {
        let table = table::TableRow::new(x, y, w, h, label);
        table.end();
        let inp = None;
        let on_update_callback: Box<dyn FnMut(i32, i32, String) + Send> = Box::new(|_, _, _| ());
        let on_update_callback = Arc::new(Mutex::new(on_update_callback));

        Self {
            table,
            inp,
            data: Default::default(),
            row_headers: Default::default(),
            col_headers: Default::default(),
            on_update_callback,
        }
    }

    /// Create a SmartTable the size of the parent widget
    pub fn default_fill() -> Self {
        Self::new(0, 0, 0, 0, None)
            .size_of_parent()
            .center_of_parent()
    }

    /// Sets the tables options
    pub fn set_opts(&mut self, opts: TableOpts) {
        let mut data = self.data.try_lock().unwrap();
        data.resize(opts.rows as _, vec![]);
        for v in data.iter_mut() {
            v.resize(opts.cols as _, Cell::default());
        }
        drop(data);

        let mut row_headers = vec![];
        for i in 0..opts.rows {
            row_headers.push((i + 1).to_string());
        }
        let row_headers = Arc::new(Mutex::new(row_headers));
        self.row_headers = row_headers;

        let mut col_headers = vec![];
        for i in 0..opts.cols {
            let mut pref = String::new();
            if i > 25 {
                let t = (i / 26) as i32;
                if t > 26 {
                    col_headers.push(i.to_string());
                } else {
                    pref.push((t - 1 + 65) as u8 as char);
                    col_headers.push(format!("{}{}", pref, (i - (26 * t) + 65) as u8 as char));
                }
            } else {
                col_headers.push(format!("{}", (i + 65) as u8 as char));
            }
        }
        let col_headers = Arc::new(Mutex::new(col_headers));
        self.col_headers = col_headers;

        let len = opts.rows;
        let inner_len = opts.cols;

        let cell = Rc::from(RefCell::from(CellData::default()));
        self.table.set_rows(len as i32);
        self.table.set_cols(inner_len as i32);
        self.table.set_row_header(true);
        self.table.set_row_resize(true);
        self.table.set_col_header(true);
        self.table.set_col_resize(true);
        self.table.end();

        // Called when the table is drawn then when it's redrawn due to events
        self.table.draw_cell({
            let cell = cell.clone();
            let data = self.data.clone();
            let row_headers = self.row_headers.clone();
            let col_headers = self.col_headers.clone();
            move |t, ctx, row, col, x, y, w, h| {
                if let Ok(data) = data.try_lock() {
                    let row_headers = row_headers.try_lock().unwrap();
                    let col_headers = col_headers.try_lock().unwrap();
                    match ctx {
                        table::TableContext::StartPage => draw::set_font(Font::Helvetica, 14),
                        table::TableContext::ColHeader => {
                            Self::draw_header(&col_headers[col as usize], x, y, w, h, &opts)
                        } // Column titles
                        table::TableContext::RowHeader => {
                            Self::draw_header(&row_headers[row as usize], x, y, w, h, &opts)
                        } // Row titles
                        table::TableContext::Cell => {
                            if t.is_selected(row, col) {
                                cell.borrow_mut().select(row, col, x, y, w, h); // Captures the cell information
                            }
                            Self::draw_data(
                                &data[row as usize][col as usize],
                                x,
                                y,
                                w,
                                h,
                                t.is_selected(row, col),
                                &opts,
                            );
                        }
                        _ => (),
                    }
                }
            }
        });

        if opts.editable {
            self.inp = Some(input::Input::default());
            let mut inp = self.inp.as_ref().unwrap().clone();
            inp.set_trigger(CallbackTrigger::EnterKey);
            let win = window::Window::from_dyn_widget_ptr(
                self.table.top_window().unwrap().as_widget_ptr(),
            );
            win.unwrap().add(&inp);
            inp.hide();

            inp.set_callback({
                let cell = cell.clone();
                let data = self.data.clone();
                let mut table = self.table.clone();
                let on_update_callback = self.on_update_callback.clone();
                move |i| {
                    let cell = cell.borrow();
                    on_update_callback.try_lock().unwrap()(cell.row, cell.col, i.value());
                    data.try_lock().unwrap()[cell.row as usize][cell.col as usize].label =
                        i.value();
                    i.set_value("");
                    i.hide();
                    table.redraw();
                }
            });

            inp.handle(|i, ev| match ev {
                Event::KeyUp => {
                    if app::event_key() == Key::Escape {
                        i.hide();
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            });

            self.table.handle({
                let data = self.data.clone();
                move |_, ev| match ev {
                    Event::Released => {
                        if let Ok(data) = data.try_lock() {
                            let cell = cell.borrow();
                            inp.resize(cell.x, cell.y, cell.w, cell.h);
                            inp.set_value(&data[cell.row as usize][cell.col as usize].label);
                            inp.show();
                            inp.take_focus().ok();
                            inp.redraw();
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            });
        }
    }

    /// Instantiate with TableOpts
    pub fn with_opts(mut self, opts: TableOpts) -> Self {
        self.set_opts(opts);
        self
    }

    /// Get the input widget
    pub fn input(&mut self) -> &mut Option<input::Input> {
        &mut self.inp
    }

    /// Get a copy of the data
    pub fn data(&self) -> CellMatrix {
        self.data.try_lock().unwrap().clone()
    }

    /// Get the inner data
    pub fn data_ref(&self) -> Arc<Mutex<CellMatrix>> {
        self.data.clone()
    }

    fn draw_header(txt: &str, x: i32, y: i32, w: i32, h: i32, opts: &TableOpts) {
        draw::push_clip(x, y, w, h);
        draw::draw_box(opts.header_frame, x, y, w, h, opts.header_color);
        draw::set_draw_color(opts.header_font_color);
        draw::set_font(opts.header_font, opts.header_font_size);
        draw::draw_text2(txt, x, y, w, h, opts.header_align);
        draw::pop_clip();
    }

    // The selected flag sets the color of the cell to a grayish color, otherwise white
    fn draw_data(cell: &Cell, x: i32, y: i32, w: i32, h: i32, selected: bool, opts: &TableOpts) {
        draw::push_clip(x, y, w, h);
        let sel_col = if let Some(sel_col) = cell.selection_color {
            sel_col
        } else {
            opts.cell_selection_color
        };
        let bg = if let Some(col) = cell.color {
            col
        } else {
            opts.cell_color
        };
        if selected {
            draw::set_draw_color(sel_col);
        } else {
            draw::set_draw_color(bg);
        }
        draw::draw_rectf(x, y, w, h);
        draw::set_draw_color(if let Some(col) = cell.font_color {
            col
        } else {
            opts.cell_font_color
        });
        draw::set_font(
            if let Some(font) = cell.font {
                font
            } else {
                opts.cell_font
            },
            if let Some(font) = cell.font_size {
                font
            } else {
                opts.cell_font_size
            },
        );
        draw::draw_text2(
            &cell.label,
            x + opts.cell_padding,
            y,
            w - opts.cell_padding * 2,
            h,
            if let Some(a) = cell.align {
                a
            } else {
                opts.cell_align
            },
        );
        draw::set_draw_color(if let Some(col) = cell.border_color {
            col
        } else {
            opts.cell_border_color
        });
        draw::draw_rect(x, y, w, h);
        draw::pop_clip();
    }

    /// Set the cell value, using the row and column to index the data
    pub fn set_cell_value(&mut self, row: i32, col: i32, val: &str) {
        self.data.try_lock().unwrap()[row as usize][col as usize].label = val.to_string();
    }

    /// Get the cell value, using the row and column to index the data
    pub fn cell_value(&self, row: i32, col: i32) -> String {
        self.data.try_lock().unwrap()[row as usize][col as usize]
            .label
            .clone()
    }

    /// Set the cell value, using the row and column to index the data
    pub fn set_cell_color(&mut self, row: i32, col: i32, color: Color) {
        self.data.try_lock().unwrap()[row as usize][col as usize].color = Some(color);
    }

    /// Set the cell value, using the row and column to index the data
    pub fn set_cell_selection_color(&mut self, row: i32, col: i32, color: Color) {
        self.data.try_lock().unwrap()[row as usize][col as usize].selection_color = Some(color);
    }

    /// Set the cell value, using the row and column to index the data
    pub fn set_cell_font_color(&mut self, row: i32, col: i32, color: Color) {
        self.data.try_lock().unwrap()[row as usize][col as usize].font_color = Some(color);
    }

    /// Set the cell value, using the row and column to index the data
    pub fn set_cell_border_color(&mut self, row: i32, col: i32, color: Color) {
        self.data.try_lock().unwrap()[row as usize][col as usize].border_color = Some(color);
    }

    /// Set the cell value, using the row and column to index the data
    pub fn set_cell_font(&mut self, row: i32, col: i32, font: Font) {
        self.data.try_lock().unwrap()[row as usize][col as usize].font = Some(font);
    }

    /// Set the cell value, using the row and column to index the data
    pub fn set_cell_font_size(&mut self, row: i32, col: i32, sz: i32) {
        self.data.try_lock().unwrap()[row as usize][col as usize].font_size = Some(sz);
    }

    /// Set the cell value, using the row and column to index the data
    pub fn set_cell_align(&mut self, row: i32, col: i32, align: Align) {
        self.data.try_lock().unwrap()[row as usize][col as usize].align = Some(align);
    }

    /// Set the row header value at the row index
    pub fn set_row_header_value(&mut self, row: i32, val: &str) {
        self.row_headers.try_lock().unwrap()[row as usize] = val.to_string();
    }

    /// Set the column header value at the column index
    pub fn set_col_header_value(&mut self, col: i32, val: &str) {
        self.col_headers.try_lock().unwrap()[col as usize] = val.to_string();
    }

    /// Get the row header value at the row index
    pub fn row_header_value(&mut self, row: i32) -> String {
        self.row_headers.try_lock().unwrap()[row as usize].clone()
    }

    /// Get the column header value at the column index
    pub fn col_header_value(&mut self, col: i32) -> String {
        self.col_headers.try_lock().unwrap()[col as usize].clone()
    }

    /// Insert an empty row at the row index
    pub fn insert_empty_row(&mut self, row: i32, row_header: &str) {
        let mut data = self.data.try_lock().unwrap();
        let cols = self.column_count() as usize;
        data.insert(row as _, vec![]);
        data[row as usize].resize(cols as _, Cell::default());
        self.row_headers
            .try_lock()
            .unwrap()
            .insert(row as _, row_header.to_string());
        self.table.set_rows(self.table.rows() + 1);
    }

    /// Append a row to your table
    pub fn insert_row(&mut self, row: i32, row_header: &str, vals: &[&str]) {
        let mut data = self.data.try_lock().unwrap();
        let cols = self.column_count() as usize;
        assert!(cols == vals.len());
        data.insert(row as _, vals.iter().map(|v| Cell::with_label(v)).collect());
        self.row_headers
            .try_lock()
            .unwrap()
            .push(row_header.to_string());
        self.table.set_rows(self.table.rows() + 1);
    }

    /// Append an empty row to your table
    pub fn append_empty_row(&mut self, row_header: &str) {
        let mut data = self.data.try_lock().unwrap();
        let cols = self.column_count() as usize;
        data.push(vec![]);
        data.last_mut().unwrap().resize(cols as _, Cell::default());
        self.row_headers
            .try_lock()
            .unwrap()
            .push(row_header.to_string());
        self.table.set_rows(self.table.rows() + 1);
    }

    /// Append a row to your table
    pub fn append_row(&mut self, row_header: &str, vals: &[&str]) {
        let mut data = self.data.try_lock().unwrap();
        let cols = self.column_count() as usize;
        assert!(cols == vals.len());
        data.push(vals.iter().map(|v| Cell::with_label(v)).collect());
        self.row_headers
            .try_lock()
            .unwrap()
            .push(row_header.to_string());
        self.table.set_rows(self.table.rows() + 1);
    }

    /// Insert an empty column at the column index
    pub fn insert_empty_col(&mut self, col: i32, col_header: &str) {
        let mut data = self.data.try_lock().unwrap();
        for v in data.iter_mut() {
            v.insert(col as _, Cell::default());
        }
        self.col_headers
            .try_lock()
            .unwrap()
            .insert(col as _, col_header.to_string());
        self.table.set_cols(self.table.cols() + 1);
    }

    /// Append a column to your table
    pub fn insert_col(&mut self, col: i32, col_header: &str, vals: &[&str]) {
        let mut data = self.data.try_lock().unwrap();
        assert!(vals.len() == self.table.rows() as usize);
        let mut count = 0;
        for v in data.iter_mut() {
            v.insert(col as _, Cell::with_label(vals[count]));
            count += 1;
        }
        self.col_headers
            .try_lock()
            .unwrap()
            .push(col_header.to_string());
        self.table.set_cols(self.table.cols() + 1);
    }

    /// Append an empty column to your table
    pub fn append_empty_col(&mut self, col_header: &str) {
        let mut data = self.data.try_lock().unwrap();
        for v in data.iter_mut() {
            v.push(Cell::default());
        }
        self.col_headers
            .try_lock()
            .unwrap()
            .push(col_header.to_string());
        self.table.set_cols(self.table.cols() + 1);
    }

    /// Append a column to your table
    pub fn append_col(&mut self, col_header: &str, vals: &[&str]) {
        let mut data = self.data.try_lock().unwrap();
        assert!(vals.len() == self.table.rows() as usize);
        let mut count = 0;
        for v in data.iter_mut() {
            v.push(Cell::with_label(vals[count]));
            count += 1;
        }
        self.col_headers
            .try_lock()
            .unwrap()
            .push(col_header.to_string());
        self.table.set_cols(self.table.cols() + 1);
    }

    /// Remove a row at the row index
    pub fn remove_row(&mut self, row: i32) {
        let mut data = self.data.try_lock().unwrap();
        data.remove(row as _);
        self.row_headers.try_lock().unwrap().remove(row as _);
        self.table.set_rows(self.table.rows() - 1);
    }

    /// Remove a column at the column index
    pub fn remove_col(&mut self, col: i32) {
        let mut data = self.data.try_lock().unwrap();
        for v in data.iter_mut() {
            v.remove(col as _);
        }
        self.col_headers.try_lock().unwrap().remove(col as _);
        self.table.set_cols(self.table.cols() - 1);
    }

    /// Set a callback for the SmartTable
    pub fn set_callback<F: FnMut(&mut Self) + 'static>(&mut self, mut cb: F) {
        let mut s = self.clone();
        self.table.set_callback(move |_| {
            cb(&mut s);
        });
    }

    /// Set a callback for on user input
    /// callback function takes the values row, col, and the new value of the cell
    pub fn set_on_update_callback<F: FnMut(i32, i32, String) + Send + 'static>(&mut self, cb: F) {
        *self.on_update_callback.try_lock().unwrap() = Box::new(cb);
    }

    /// Clears all cells in the table
    pub fn clear(&mut self) {
        let mut data = self.data.try_lock().unwrap();
        for v in data.iter_mut() {
            for c in v.iter_mut() {
                *c = Cell::default();
            }
        }
    }

    /// Returns the row count
    pub fn row_count(&self) -> i32 {
        self.table.rows()
    }

    /// Returns the column count
    pub fn column_count(&self) -> i32 {
        self.table.cols()
    }

    /// Get the column's width
    pub fn col_width(&self, col: i32) -> i32 {
        self.table.col_width(col)
    }

    /// Get the row's height
    pub fn row_height(&self, row: i32) -> i32 {
        self.table.row_height(row)
    }

    /// Set column's width
    pub fn set_col_width(&mut self, col: i32, width: i32) {
        self.table.set_col_width(col, width);
    }

    /// Set the row's height
    pub fn set_row_height(&mut self, row: i32, height: i32) {
        self.table.set_row_height(row, height);
    }

    /// Get the column header height
    pub fn col_header_height(&self) -> i32 {
        self.table.col_header_height()
    }

    /// Get the row header width
    pub fn row_header_width(&self) -> i32 {
        self.table.row_header_width()
    }

    /// Set column header height
    pub fn set_col_header_height(&mut self, height: i32) {
        self.table.set_col_header_height(height);
    }

    /// Set the row header width
    pub fn set_row_header_width(&mut self, width: i32) {
        self.table.set_row_header_width(width);
    }
}

impl std::fmt::Debug for SmartTable {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("SmartTable")
            .field("table", &self.table)
            .field("data", &self.data)
            .field("row_headers", &self.row_headers)
            .field("col_headers", &self.col_headers)
            .finish()
    }
}

fltk::widget_extends!(SmartTable, table::TableRow, table);
