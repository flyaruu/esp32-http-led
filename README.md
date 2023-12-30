# esp32-http-led
This is a repo to show how to drive an amoled display using the ESP32S3. The display is controllable via Wi-Fi.
## Getting started
To get started using this repo, first follow [these instructions](https://esp-rs.github.io/book/installation/riscv-and-xtensa.html)

Then flash the code using:
```
cargo run --release
```

Returns 'hello world' from a GET /, you can test it with curl:
```
curl http://<ESP_IP_ADDRESS/
```

To get the display to change POST to /shape with JSON data:
```
curl -v -d '{"Triangle":{"a":{"x":150,"y": 340},"b":{"x":220,"y":460},"c":{"x":150,"y":530}}}' http://<ESP_IP_ADDRESS>/shape
```
and adjust the values.

Meant to be forked into different projects.

[There is also a youtube video, following the development process of this repo](https://www.youtube.com/watch?v=l8VCC-XhTcs&list=PL0U7YUX2VnBFbwTi96wUB1nZzPVN3HzgS&index=18)
