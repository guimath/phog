# Phog - Photo gallery
![Logo phog](ui/img/logo.svg) 

Phog is an app built in Rust that allows for quick sorting of files for photographers shooting in JPEG+RAW.

It can also be used as simple photo gallery to browse through your photos with minimal loading time.

## Install  

- Install Rust by following the [Rust Getting Started Guide](https://www.rust-lang.org/learn/get-started)
- Install the app from the rust crates library with  `cargo install phog` or install it directly from this github by cloning and then using `cargo install phog --path .`

## Using 

In the terminal, navigate to the folder of photos you want to see, and launch the app with `phog` (some parameters are available from the command line, type `phog -h` for more). This will scan the current directory for photos, if any are found they will be loaded and you can look through them.

- Navigate the images with the arrows 
- You can copy an image (and it's raw) to a separate "edit" folder by pressing `e`. 
- You can move an image (and it's raw) to a "bin" folder by pressing `d` 
    
    (currently this is simply a subfolder of the current folder, if you wan't to fully delete it, delete the folder once you're done). 

For more information, press `h` to display the help.
