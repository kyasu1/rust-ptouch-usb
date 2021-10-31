# ptouch

This crate provides raw printing capability for Brother P-Touch QL series label printers connected via USB port. It also provides utility functions to convert PNG images to printer friendly format including two-color and high-resolutiong formats.

The label data sent to printers is respresented as a two dimensional array, `Vec<Vec<u8>>`, each row represents a rastered line data. 

The label data sent to printer are represented by a collection of binary data which implemnts `Iterator` trait with lazy generation. The label data is respresented as a two dimensional array, Vec<Vec<u8>>, conversion from another image formats can be easily done.

## Features

- Allow printing multiple devices connected through USB ports at same time.
- Print multiple labels at once.
- High resolution printing support.
- [] Two colors printing support.

## Usage

### Media Tape

Theare are two kinds of media tapes, Continuous and DieCut. Each one has several size variations. The supported media types are follows.

```
    Continuous12,
    Continuous29,
    Continuous38,
    Continuous50,
    Continuous54,
    Continuous62,
    Continuous62Red,
```    

Install a media tape to your printer, then specify the matching type. In this example we choose Continous tape with 62mm width.

```rust
let media = ptouch::Media::Continous(ContinuousType::Continuous62);
```

### PTouch device model and serial number

You can inspect USB ports by `lsusb -v` which shows something like below, where `iProduct` and `iSerial` are what we need.

```
Bus 001 Device 003: ID 04f9:209b Brother Industries, Ltd 
Device Descriptor:
  bLength                18
  bDescriptorType         1
  bcdUSB               2.00
  bDeviceClass            0 (Defined at Interface level)
  bDeviceSubClass         0 
  bDeviceProtocol         0 
  bMaxPacketSize0        64
  idVendor           0x04f9 Brother Industries, Ltd
  idProduct          0x209b 
  bcdDevice            1.00
  iManufacturer           1 Brother
  iProduct                2 QL-800
  iSerial                 3 000G0Z000000
```

With these information we can intialize our configurations as follows.

```rust
let config: Config = Config::new(Model::QL800, "000G0Z000000".to_string(), media)
	.high_resolution(false)
	.cut_at_end(true)
	.two_colors(false)
	.enable_auto_cut(1);
```

These are default settings, `high_resolution` and `two_colors` options work but you need to provide appropriate data.

### Label Image Data

This part is tricky, since this crate provides only printing capabilities, label data must be prepared with compatible format. As shown in the PTouch printer documentation, QL series expects image data with an 1-bit indexed bitmap split by lines in an appropriate orders. Please see the document for more detail.

In this example we create an image data with a size of 720 x 400 pixel by using an application. Then using `image` crate to read and convert it to a grayscale data. Then using `step_filter_normal` function, which is supplied with this crate, we binalize and pack bits 90 bytes width vector data.

```rust
let file = "examples/rust-logo.png";
let image: image::DynamicImage = image::open(file).unwrap();
let (_, length) = image.dimensions();
let gray = image.grayscale();
let mut buffer = image::DynamicImage::new_luma8(ptouch::WIDE_PRINTER_WIDTH, length);
buffer.invert();
buffer.copy_from(&gray, 0, 0).unwrap();
buffer.invert();
let bytes = buffer.to_bytes();
let bw = ptouch::utils::step_filter_normal(80, length, bytes);
```

#### Tips for creating data

In this crate, the width of image data must be 720px, which is the number of pins the printer have. The length varies depending on the label media. For the DieCut labels, there is a specif value. In case of the Continous labels, you can choose any length between 150px to 11811px for normal resolution (for 300 dpi). If you are specifying high_resolution or two_clolors options, it must be halved. After determing the size, place your contets in the area where actual labels go through. If you are using 62mm media, full width will be printed. But for 29mm media, you need to give an offset of 408 pixel on the left side then place content in 306 pixel width. You can check the details of media specification in the manual.

### Printing

Once you get the bitmap data, you can supply them as a Vec.

```rust
match Printer::new(config) {
    Ok(printer) => {
		printer.print(vec![bw.clone()]).unwrap();
    }
    Err(err) => panic!("Printer Error {}", err),
}
```

If the configuration value is invalid the `new` function will return an error.

Note: When sending a long label, at maximum it can be 1000mm long, rusb will timeout and casue error. 

## Supported Printers

The following models are tested by myself. 

- QL-720NW
- QL-800
- QL-820NWB

Anothre printers listed in the `ptouch::Model` should also work but we might need to some tweeking.

## Tools

In the example, there is a small tool to read the printer status.

```
RUST_LOG=debug cargo run --example read_status
```

This will show something like follows.

```
[2020-10-27T06:05:45Z DEBUG ptouch::printer] Raw status code: [80, 20, 42, 34, 38, 30, 0, 0, 0, 0, 1D, A, 0, 0, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]
[2020-10-27T06:05:45Z DEBUG ptouch::printer] Parsed Status struct: Status { model: QL800, error: UnknownError(0), media: Some(Continuous(Continuous29)), mode: 0, status_type: ReplyToRequest, phase: Receiving, notification: NotAvailable }
```

Some gathered data are saved in `printer_status.txt`.

## Todos

- [] Better error handling and reporting, what to do when label ends ?
- [] Binalization with dithering support
- [] Binalization with two colors support

## Tips

### Allow access to USB port on ubuntu

For my system with ubuntu 18.04 on Raspberry Pi 4, I got an error about permission to `/dev/usb/lp0`. To fix that I needed to add something like follows.

Add the current user to `plugdev` group.

```sh
$ sudo gpasswd -a $USER plugdev
```

Add a new rule for udev. Here the number of filename must be higher than 60 to apply its config(Wasted whole day to notice abouth that >.<).

```sh:/etc/udev/rules.d/65-ptouch.rules
SUBSYSTEMS=="usb", ATTRS{idVendor}=="04f9", ATTRS{idProduct}=="209b|209c|209d", MODE="664", GROUP="plugdev"
```

Reload the rules.

```
sudo udevadm trigger
sudo udevadm control --reload-rules
```

https://static.dev.sifive.com/dev-tools/FreedomStudio/2019.08/freedom-studio-manual-4.7.2-2019-08-1.pdf

https://gill.net.in/posts/reverse-engineering-a-usb-device-with-rust/


## License

MIT



```
use ptouch_rs::{Printer, PrinterProfile, Model, Media};

let file = "examples/assets/label-720-300.png";
let label: image::DynamicImage = image::open(file).unwrap().grayscale();
let (_, length) = label.dimensions();
let bytes = label.to_bytes();
let bw = step_filter_normal(80, length, bytes);

let printer = Printer::new(profile, media)
    .high_resolution(false)
    .cut_at_end(true)
    .two_colors(false)
    .enable_auto_cut(1);

printer.print(vec![bw].into_iter()).unwrap();
```