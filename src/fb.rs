#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
pub(crate) struct FbBitField {
    pub offset: u32,      /* beginning of bitfield */
    pub length: u32,      /* length of bitfield */
    pub msb_right: u32,   /* !=0: Most significant bit is right */
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
pub(crate) struct FbVarScreenInfo {
    pub xres: u32,           /* visible resolution       */
    pub yres: u32,
    pub xres_virtual: u32,   /* virtual resolution       */
    pub yres_virtual: u32,
    pub xoffset: u32,        /* offset from virtual to visible */
    pub yoffset: u32,        /* resolution           */

    pub bits_per_pixel: u32, /* guess what           */
    pub grayscale: u32,      /* != 0 Graylevels instead of colors */

    pub red: FbBitField,     /* bitfield in fb mem if true color, */
    pub green: FbBitField,   /* else only length is significant */
    pub blue: FbBitField,
    pub transp: FbBitField,  /* transparency         */  

    pub nonstd: u32,         /* != 0 Non standard pixel format */

    pub activate: u32,       /* see FB_ACTIVATE_*        */

    pub height: u32,         /* height of picture in mm    */
    pub width: u32,          /* width of picture in mm     */

    pub accel_flags: u32,    /* (OBSOLETE) see fb_info.flags */

    /* Timing: All values in pixclocks, except pixclock (of course) */
    pub pixclock: u32,       /* pixel clock in ps (pico seconds) */
    pub left_margin: u32,    /* time from sync to picture    */
    pub right_margin: u32,   /* time from picture to sync    */
    pub upper_margin: u32,   /* time from sync to picture    */
    pub lower_margin: u32,
    pub hsync_len: u32,      /* length of horizontal sync    */
    pub vsync_len: u32,      /* length of vertical sync  */
    pub sync: u32,           /* see FB_SYNC_*        */
    pub vmode: u32,          /* see FB_VMODE_*       */
    pub rotate: u32,         /* angle we rotate counter clockwise */
    pub reserved: [u32; 5]   /* Reserved for future compatibility */
}

#[repr(C)]
#[derive(Default, Debug)]
pub(crate) struct FbFixScreenInfo {
    pub id: [u8; 16],       /* identification string eg "TT Builtin" */
    pub smem_start: usize,    /* Start of frame buffer mem */
                        /* (physical address) */
    pub smem_len: u32,      /* Length of frame buffer mem */
    pub type_: u32,         /* see FB_TYPE_*        */
    pub type_aux: u32,      /* Interleave for interleaved Planes */
    pub visual: u32,        /* see FB_VISUAL_*      */ 
    pub xpanstep: u16,      /* zero if no hardware panning  */
    pub ypanstep: u16,      /* zero if no hardware panning  */
    pub ywrapstep: u16,     /* zero if no hardware ywrap    */
    pub line_length: u32,   /* length of a line in bytes    */
    pub mmio_start: usize,    /* Start of Memory Mapped I/O   */
                        /* (physical address) */
    pub mmio_len: u32,      /* Length of Memory Mapped I/O  */
    pub accel: u32,         /* Indicate to driver which */
                        /*  specific chip/card we have  */
    pub reserved: [u16; 3]  /* Reserved for future compatibility */
}

#[repr(C)]
#[derive(Default, Debug)]
pub(crate) struct MxcfbRect {
    pub top: u32,
    pub left: u32,
    pub width: u32,
    pub height: u32
}

#[repr(C)]
#[derive(Default, Debug)]
pub(crate) struct MxcfbAltBufferData {
    pub phys_addr: u32,
    pub width: u32,
    pub height: u32,
    pub alt_update_region: MxcfbRect
}

#[repr(C)]
#[derive(Default, Debug)]
pub(crate) struct MxcfbUpdateData51 {
    pub update_region: MxcfbRect,
    pub waveform_mode: u32,
    pub update_mode: u32,
    pub update_marker: u32,
    pub hist_bw_waveform_mode: u32,
    pub hist_gray_waveform_mode: u32,
    pub temp: i32,
    pub flags: u32,
    pub alt_buffer_data: MxcfbAltBufferData
}
