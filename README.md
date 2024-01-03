# PushToTalk
A simple program that unmutes the default microphone if a specific button is pressed.
Currently this is handled by a global variable which is set to CAPSLOCK:
```
const PUSH_TO_TALK_KEY: Key = Key::KEY_CAPSLOCK;
```
