/// total number of images loaded in buffer
const BUFFER_SIZE: usize = 8;

/// Minimum number of elements to carry on either side of the buffer
const MIN_ELEM_NUM: usize = (BUFFER_SIZE - 2) / 2;

use std::sync::Arc;

use exif::Tag;
use slint::{Image, Rgb8Pixel, SharedPixelBuffer};
use std::fs;
use std::time::Instant;
use std::{cmp::min, path::PathBuf};
use tokio::spawn;
use tokio::sync::Mutex;
use turbojpeg::image::Rgb;
// use image::{ImageReader, Rgb8Pixel};
// use image::SharedPixelBuffer;
use turbojpeg::{Transform, TransformOp};

/// Image element with logic to load and read data as fast as possible
#[derive(Debug, Clone)]
struct ImageElement {
    raw_img: SharedPixelBuffer<Rgb8Pixel>,
    file_name: String,
}
impl Default for ImageElement {
    fn default() -> Self {
        // const BASE_WIDTH:u32 = 6000;
        // const BASE_HEIGHT:u32 = 4000;
        // cannot statically allocate correct size because size will be >> than stack size
        // can be run in thread with changed stack size but that's a lot of hassle
        Self {
            raw_img: SharedPixelBuffer::new(1, 1),
            file_name: String::default(),
        }
    }
}

impl ImageElement {
    pub fn read(&self) -> Image {
        Image::from_rgb8(self.raw_img.clone())
    }

    pub fn load(&mut self, elem: PathBuf) {
        self.file_name = elem.file_name().unwrap().to_str().unwrap().to_string();
        // reading EXIF to get orient (<10ms)
        let file = fs::File::open(elem.clone()).unwrap();
        let mut buffer_reader = std::io::BufReader::new(&file);
        let exif_reader = exif::Reader::new();
        let exif_res = exif_reader.read_from_container(&mut buffer_reader);
        let mut rotate = None;
        if let Ok(exif) = exif_res {
            if let Some(orientation) = exif.get_field(Tag::Orientation, exif::In::PRIMARY) {
                rotate = match orientation.value.get_uint(0) {
                    Some(1) => None,                                     // in landscape
                    Some(3) => Some(Transform::op(TransformOp::Rot180)), // in landscape upside down
                    Some(6) => Some(Transform::op(TransformOp::Rot90)),  // in portrait
                    Some(8) => Some(Transform::op(TransformOp::Rot270)), // in portrait flipped
                    _ => None, // Could not determine orientation based on EXIF data
                }
            }
        }

        // decoding and rotating if needed (100ms + 400ms)
        let data = fs::read(elem).unwrap();
        let decoded = if let Some(transform) = rotate {
            let o = turbojpeg::transform(&transform, &data).unwrap();
            turbojpeg::decompress_image::<Rgb<u8>>(&o).unwrap()
        } else {
            turbojpeg::decompress_image::<Rgb<u8>>(&data).unwrap()
        };

        // if self.raw_img.width() == decoded.height() && self.raw_img.height() == decoded.width() {
        //     // Same size so buffer stays the same but inverts width and height -> awaiting slint update
        //     self.raw_img = SharedPixelBuffer::from_shared_pixel_buffer(decoded.width(), decoded.height(), self.raw_img.clone()).unwrap();
        // } else
        if self.raw_img.width() != decoded.width() || self.raw_img.height() != decoded.height() {
            self.raw_img =
                SharedPixelBuffer::clone_from_slice(&decoded, decoded.width(), decoded.height());
            return;
        }

        // Actual copying rather fast (10ms)
        let img_data = self.raw_img.make_mut_bytes();
        unsafe {
            std::ptr::copy_nonoverlapping(decoded.as_ptr(), img_data.as_mut_ptr(), decoded.len());
        }
    }
}

pub struct ImageStat {
    pub image: Image,
    pub name: String,
    pub number: usize,
    pub out_of: usize,
}

/// A circular buffer implementation that aims to center a current element but keep in memory
/// a few elements before and after
/// You can move through the elements in both directions
pub struct CircularBuffer {
    /// current item counter in global elem list (pic_list)
    counter: usize,
    /// list of all path to rotate between
    pic_list: Vec<PathBuf>,
    /// Actual buffered elements
    buffer: [Arc<Mutex<ImageElement>>; BUFFER_SIZE],
    /// true size of buffer (might be smaller than BUFFER_SIZE if total amount of elements are smaller)
    true_size: usize,
    /// used indices of the buffer array
    /// to avoid unnecessary copies when an item is deleted and there is not enough items left to fill
    indices: Vec<usize>,
    /// index of the current element
    current_idx: usize,
    /// number of elements in buffer in front of current
    front_file: usize,
    /// number of elements in buffer back of current
    back_file: usize,
}

impl CircularBuffer {
    pub fn new(pic_list: Vec<PathBuf>) -> Self {
        let true_size = min(BUFFER_SIZE, pic_list.len());
        let indices: Vec<usize> = (0..true_size).collect();
        let a = Instant::now();
        let buffer = [(); BUFFER_SIZE].map(|_| Arc::new(Mutex::new(ImageElement::default())));
        buffer[0].blocking_lock().load(pic_list[0].clone());

        println!("First img loaded ({:?})", Instant::now() - a);
        Self {
            counter: 0,
            pic_list,
            buffer,
            true_size,
            indices,
            current_idx: 0,
            front_file: true_size - 1,
            back_file: 0,
        }
    }

    /// helper fn to increase by one current_idx
    fn incr_idx(&mut self) {
        self.current_idx = (self.current_idx + 1) % self.true_size
    }
    /// helper fn to decrease by one current_idx
    fn decr_idx(&mut self) {
        self.current_idx = (self.current_idx + self.true_size - 1) % self.true_size
    }
    fn current_buffer_idx(&self) -> usize {
        self.indices[self.current_idx]
    }
    fn front_buffer_idx(&self) -> usize {
        self.indices[(self.current_idx + self.front_file) % self.true_size]
    }
    fn back_buffer_idx(&self) -> usize {
        self.indices[(self.current_idx + self.true_size - self.back_file) % self.true_size]
    }

    /// Spawns loads on all buffer in parallel
    pub async fn init(&mut self) {
        for i in 1..self.true_size {
            self.load(self.pic_list[i].clone(), i).await;
        }
    }

    /// Switches current element to next one. Also launches a load if needed
    pub async fn next_img(&mut self) -> bool {
        if self.counter == self.pic_list.len() - 1 {
            return false;
        }
        self.incr_idx();
        self.counter += 1;

        let front_buf_full = self.front_file > MIN_ELEM_NUM;
        let no_more_elems = self.counter + MIN_ELEM_NUM >= self.pic_list.len();
        if front_buf_full || no_more_elems {
            self.front_file -= 1;
            self.back_file += 1;
        } else {
            self.load_elem_front().await;
        }
        true
    }

    /// Switches current element to previous one. Also launches a load if needed
    pub async fn prev_img(&mut self) -> bool {
        if self.counter == 0 {
            return false;
        }
        self.decr_idx();
        self.counter -= 1;
        let back_buf_full = self.back_file > MIN_ELEM_NUM;
        let no_more_elems = self.counter < self.back_file;
        if back_buf_full || no_more_elems {
            self.back_file -= 1;
            self.front_file += 1;
        } else {
            self.load_elem_back().await;
        }
        true
    }

    async fn load_elem_back(&mut self) {
        let elem = self.pic_list[self.counter - self.back_file].clone();
        self.load(elem, self.back_buffer_idx()).await;
    }
    async fn load_elem_front(&mut self) {
        let elem = self.pic_list[self.counter + self.front_file].clone();
        self.load(elem, self.front_buffer_idx()).await;
    }
    async fn load(&mut self, elem: PathBuf, buf_pos: usize) {
        let a = Arc::clone(&self.buffer[buf_pos]);
        spawn(async move {
            let mut a_lock = a.lock().await;
            a_lock.load(elem);
        });
    }

    /// deletes current element and launches load on new element if possible
    pub async fn delete(&mut self) -> bool {
        self.pic_list.remove(self.counter);
        if self.pic_list.is_empty() {
            println!("No more photos, everything in the folder was deleted");
            return false;
        }

        let buf_idx = self.current_buffer_idx();
        if self.pic_list.len() < BUFFER_SIZE {
            // nothing to fill buffer with -> removing current buf idx from indices
            self.indices.retain(|value| *value != buf_idx);
            self.true_size -= 1;
            if self.front_file == 0 {
                self.counter -= 1;
                self.back_file -= 1;
                self.decr_idx();
                return true;
            }
            self.front_file -= 1;
            return true;
        }
        if self.front_file == 0 {
            //          |          |
            // 1, 2, 3, 4 -> 1, 2, 3, 0
            self.decr_idx();
            self.counter -= 1;
            self.load_elem_back().await;
            return true;
        }

        //       |                |
        // 1, 2, 3, 4, 5 -> 1, 2, 4, 5, 6
        for i in 0..self.front_file {
            self.indices[(self.current_idx + i) % self.true_size] =
                self.indices[(self.current_idx + i + 1) % self.true_size];
        }
        self.indices[(self.current_idx + self.front_file) % self.true_size] = buf_idx;

        let all_front_loaded = self.counter + MIN_ELEM_NUM >= self.pic_list.len();
        if all_front_loaded {
            self.front_file -= 1;
            self.back_file += 1;
            self.load_elem_back().await;
        } else {
            // need to fill from the front
            self.load_elem_front().await;
        }
        true
    }

    /// reading element and returning stats
    pub async fn get_elem(&self) -> ImageStat {
        if false {
            let mut before: Vec<String> = Vec::new();
            let mut after: Vec<String> = Vec::new();
            for i in 0..self.back_file {
                let buf_num = self.indices
                    [(self.current_idx + self.true_size + i - self.back_file) % self.true_size];
                before.push(self.buffer[buf_num].lock().await.file_name.clone())
            }
            for i in 0..self.front_file {
                let buf_num = self.indices[(self.current_idx + i + 1) % self.true_size];
                after.push(self.buffer[buf_num].lock().await.file_name.clone())
            }
            #[rustfmt::skip]
            println!("{:?}, {} {:?} (real: {} | buf_num: {})", before, self.buffer[self.current_buffer_idx()].lock().await.file_name, after, self.counter, self.current_buffer_idx());
            println!("{:?}", self.indices)
        }
        let elem = self.buffer[self.current_buffer_idx()].lock().await;
        ImageStat {
            image: elem.read(),
            name: elem.file_name.clone(),
            number: self.counter + 1,
            out_of: self.pic_list.len(),
        }
    }

    /// non async function to get the first imageStat at initialization.
    pub fn get_first_elem(&self) -> ImageStat {
        let elem = self.buffer[0].blocking_lock();
        ImageStat {
            image: elem.read(),
            name: elem.file_name.clone(),
            number: self.counter + 1,
            out_of: self.pic_list.len(),
        }
    }
}
