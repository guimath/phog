/// total number of images loaded in buffer
const BUFFER_SIZE :usize = 6;

/// Minimum number of elements to carry on either side of the buffer
const MIN_ELEM_NUM:usize = (BUFFER_SIZE-2)/2;

use std::{cmp::min, path::PathBuf};
use std::fs;
use std::time::Instant;
use slint::{Image, Rgb8Pixel, SharedPixelBuffer};
use turbojpeg::image::Rgb;
use exif::Tag;
// use image::{ImageReader, Rgb8Pixel};
// use image::SharedPixelBuffer;
use turbojpeg::{Transform, TransformOp};


const BASE_WIDTH:u32 = 6000;
const BASE_HEIGHT:u32 = 4000;

#[derive(Debug, Clone)]
struct ImageElement{
    raw_img : SharedPixelBuffer<Rgb8Pixel>,
    raw_img_portrait : SharedPixelBuffer<Rgb8Pixel>,
    file_name : String,
    in_portrait : bool,
}

impl Default for ImageElement {
    fn default() -> Self {
        Self{
            raw_img: SharedPixelBuffer::new(1,1),
            raw_img_portrait: SharedPixelBuffer::new(1,1),
            file_name: String::default(),
            in_portrait: false,
        }
    }
}
impl ImageElement {
    #[allow(unused)]
    fn preloaded() -> Self{
        Self{
            raw_img: SharedPixelBuffer::new(BASE_WIDTH,BASE_HEIGHT),
            raw_img_portrait: SharedPixelBuffer::new(BASE_HEIGHT,BASE_WIDTH),
            file_name: String::default(),
            in_portrait: false,
        }
    }
    pub fn read(&self) -> Image{
        if self.in_portrait {
            slint::Image::from_rgb8(self.raw_img_portrait.clone())
        } else {
            slint::Image::from_rgb8(self.raw_img.clone())
        }
    }

    pub fn load(&mut self, elem:PathBuf){
        self.file_name = elem.file_name().unwrap().to_str().unwrap().to_string();
        // reading EXIF to get orient (<10ms)
        let file = fs::File::open(elem.clone()).unwrap();
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = exifreader.read_from_container(&mut bufreader).unwrap();
        let rotate = if let Some(orientation) = exif.get_field(Tag::Orientation, exif::In::PRIMARY) {
            match orientation.value.get_uint(0) {
                Some(1) => None,  // in landscape
                Some(3) => Some(Transform::op(TransformOp::Rot180)),// in landscape upside down
                Some(6) => Some(Transform::op(TransformOp::Rot90)), // in portrait 
                Some(8) => Some(Transform::op(TransformOp::Rot270)),// in portrait flipped
                _ => None, // Could not determine orientation based on EXIF data
            }
        } else {
            None
        };

        // decoding and rotating if needed (100ms + 400ms)
        let data= fs::read(elem).unwrap();
        let decoded = if let Some(transform) = rotate {
            let o = turbojpeg::transform(&transform, &data).unwrap();
            turbojpeg::decompress_image::<Rgb<u8>>(&o).unwrap()
        } else {
            turbojpeg::decompress_image::<Rgb<u8>>(&data).unwrap()
        };
        
        // SharedPixelBuffer creation can take a while on large files (1s)
        self.in_portrait = decoded.width()< decoded.height();

        let image = if self.in_portrait {
            &mut self.raw_img_portrait
        } else {
            &mut self.raw_img
        };

        if image.width() != decoded.width() || image.height() != decoded.height() { 
            // only recreating if not same size
            *image = SharedPixelBuffer::new(decoded.width(), decoded.height());
        }
        let img_data=  image.make_mut_bytes();
        unsafe {
            std::ptr::copy_nonoverlapping(decoded.as_ptr(), img_data.as_mut_ptr(), decoded.len());
        }        
    }
}


pub struct CircularBuffer {
    counter: usize,
    pic_list: Vec<PathBuf>,
    /// Actual bufferized elements
    buffer: [ImageElement; BUFFER_SIZE],
    /// true size of buffer (might be smaller than BUFFER_SIZE if total amount of elements are smaller)
    true_size: usize,
    /// used indices of the buffer array 
    /// to avoid unnecessary copies when an item is deleted and there is not enough items left to fill
    indices: Vec<usize>,
    /// index of the current element 
    current_idx: usize,
    /// number of elements in buffer in front of current
    front_file:usize,
    /// number of elements in buffer back of current
    back_file:usize,
}

impl CircularBuffer {
    pub fn new(pic_list:Vec<PathBuf>) -> Self {
        
        let true_size= min(BUFFER_SIZE, pic_list.len());
        let indices: Vec<usize> = (0..true_size).collect();
        let a = Instant::now();
        let buffer: [ImageElement; BUFFER_SIZE] = [(); BUFFER_SIZE].map(|_| ImageElement::default());
        println!("Space allocated ({:?})", Instant::now()-a);
        Self{
            counter: 0,
            pic_list,
            buffer,
            true_size,
            indices,
            current_idx: 0,
            front_file: true_size-1,
            back_file: 0,
        }
    }

    pub fn init(&mut self) {
        for i in 0..self.true_size {
            self.buffer[i].load(self.pic_list[i].clone());
        }

    }

    fn get_buffer_idx(&self, num:usize)-> usize{
        self.indices[num % self.true_size]
    }

    pub fn next_img(&mut self) -> bool {
        if self.counter == self.pic_list.len() -1 {
            return false;
        }
        self.current_idx =(self.current_idx+1)%self.true_size;
        self.counter += 1;

        let front_buf_full = self.front_file > MIN_ELEM_NUM;
        let no_more_elems = self.counter+MIN_ELEM_NUM >= self.pic_list.len();
        if front_buf_full || no_more_elems {
            self.front_file -= 1;
            self.back_file +=1;
        } else {
            self.load_elem_front();
        }
        return true;
    }

    pub fn prev_img(&mut self) -> bool {
        if self.counter == 0 {
            return false;
        }

        self.current_idx = (self.current_idx+self.true_size-1)%self.true_size;
        self.counter -= 1;
        let back_buf_full = self.back_file > MIN_ELEM_NUM;
        let no_more_elems = self.counter < self.back_file;
        if back_buf_full || no_more_elems  {
            self.back_file -= 1;
            self.front_file +=1;
        } else {
            self.load_elem_back();
        }
        true
    }

    fn load_elem_back(&mut self) {
        let elem = self.pic_list[self.counter-self.back_file].clone();
        let buf_pos = self.get_buffer_idx(self.current_idx + self.true_size - self.back_file); // avoiding negative
        self.buffer[buf_pos].load(elem);
    }

    fn load_elem_front(&mut self){
        let elem = self.pic_list[self.counter+self.front_file].clone();
        let buf_pos = self.get_buffer_idx(self.current_idx+self.front_file);
        self.buffer[buf_pos].load(elem);
    }

    // fn load_elem(&mut self, elem:PathBuf, buf_pos:usize) {

    //     self.buffer[buf_pos].file_name = elem.file_name().unwrap().to_str().unwrap().to_string();
    //     // self.buffer[buf_pos].raw_img = Image::load_from_path(elem.as_path()).expect("image read failed");
    //     // 4.2 s
    //     // let file = fs::read(elem).unwrap();
    //     let image = image::ImageReader::open(elem).unwrap().decode().unwrap();
    //     // let rgb_buf = image::load_from_memory(&file.as_bytes()).unwrap().into_rgb8(); 
    //     let shared_buf: SharedPixelBuffer<Rgb8Pixel> = SharedPixelBuffer::clone_from_slice(image.as_rgb8().unwrap(), image.width(), image.height()); 
    //     let a = Instant::now();
    //     self.buffer[buf_pos].raw_img = slint::Image::from_rgb8(shared_buf); 
    //     println!("{:?}", Instant::now() - a);
    // }

    pub fn delete(&mut self) -> bool {
        self.pic_list.remove(self.counter);
        let buf_num = self.get_buffer_idx(self.current_idx);
        if self.pic_list.len() == 0 {
            println!("No more photos, everything in the folder was deleted");
            return false;
        }
        if self.pic_list.len() < BUFFER_SIZE {
            self.indices.retain(|value| *value != buf_num);
            self.true_size -=1;
            if self.front_file == 0 {
                self.counter -= 1;
                self.back_file -=1;
                self.current_idx -=1;
                return true;
            }
            self.front_file -=1;
            return true;
        }
        if self.front_file == 0 {
            for i in 0..self.back_file {
                let buf_num1 = self.get_buffer_idx(self.current_idx+self.true_size-i);
                let buf_num2 = self.get_buffer_idx(self.current_idx+self.true_size-i-1);
                self.buffer[buf_num1] = self.buffer[buf_num2].clone();
                // *self.pic_buffer[buf_num1].lock().unwrap() = self.pic_buffer[buf_num2].lock().unwrap().clone();
            }
            self.counter -= 1;
            self.load_elem_back();
            return true;
        }
        for i in 0..self.front_file {
            let buf_num1 = self.get_buffer_idx(self.current_idx+i);
            let buf_num2 = self.get_buffer_idx(self.current_idx+i+1);
            self.buffer[buf_num1] = self.buffer[buf_num2].clone();
        }

        if self.counter+MIN_ELEM_NUM >= self.pic_list.len() {
            self.front_file -= 1;
            self.back_file +=1;
            self.load_elem_back();
        }
        else {
            self.load_elem_front();
        }
        true
    }


    pub fn get_elem(&self) -> Image {
        if false {
            let mut before: Vec<String> = Vec::new();
            let mut after: Vec<String> = Vec::new();
            for i in 0..self.back_file {
                let buf_num = self.get_buffer_idx(self.current_idx + self.true_size+  i - self.back_file);
                before.push(self.buffer[buf_num].file_name.clone())
            }
            for i in 0..self.front_file {
                let buf_num = self.get_buffer_idx(self.current_idx + i+1);
                after.push(self.buffer[buf_num].file_name.clone())
            }
            println!("{:?}, {} {:?} (real: {} | buf_num: {})", before, self.buffer[self.get_buffer_idx(self.current_idx)].file_name, after, self.counter, self.get_buffer_idx(self.current_idx));
            // println!("{} real: {}", self.buffer[self.current_idx].file_name, self.counter);
        }
        // self.fut_buffer[self.get_buffer_idx(self.buffer_num)].as_mut().await;

        self.buffer[self.get_buffer_idx(self.current_idx)].read()
    }

    pub fn get_elem_infos(&self)-> (String, usize, usize){
        (self.buffer[self.get_buffer_idx(self.current_idx)].file_name.clone(), self.counter+1, self.pic_list.len())
    }


}