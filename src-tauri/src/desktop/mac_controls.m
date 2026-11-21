#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#import <AudioToolbox/AudioHardwareService.h>
#import <CoreAudio/CoreAudio.h>
#import <IOKit/hidsystem/ev_keymap.h>

// The shared shortcut parser returns virtual key codes. Convert them to Quartz
// physical key positions here, matching KeyboardEvent.code in the recorder.
static int keyCode(uint16_t vk) {
    static const int letters[] = {0, 11, 8, 2, 14, 3, 5, 4, 34, 38, 40, 37, 46,
                                  45, 31, 35, 12, 15, 1, 17, 32, 9, 13, 7, 16, 6};
    static const int digits[] = {29, 18, 19, 20, 21, 23, 22, 26, 28, 25};
    static const int functionKeys[] = {122, 120, 99, 118, 96, 97, 98, 100, 101, 109,
                                       103, 111, 105, 107, 113, 106, 64, 79, 80, 90};
    if (vk >= 'A' && vk <= 'Z') return letters[vk - 'A'];
    if (vk >= '0' && vk <= '9') return digits[vk - '0'];
    if (vk >= 0x70 && vk <= 0x83) return functionKeys[vk - 0x70];
    switch (vk) {
        case 0x08: return 51;  // backspace
        case 0x09: return 48;  // tab
        case 0x0d: return 36;  // return
        case 0x1b: return 53;  // escape
        case 0x20: return 49;  // space
        case 0x21: return 116; // page up
        case 0x22: return 121; // page down
        case 0x23: return 119; // end
        case 0x24: return 115; // home
        case 0x25: return 123; // left
        case 0x26: return 126; // up
        case 0x27: return 124; // right
        case 0x28: return 125; // down
        case 0x2e: return 117; // forward delete
        default: return -1;
    }
}

static CGEventFlags modifier(uint16_t vk) {
    switch (vk) {
        case 0x10: return kCGEventFlagMaskShift;
        case 0x11: return kCGEventFlagMaskControl;
        case 0x12: return kCGEventFlagMaskAlternate;
        case 0x5b: return kCGEventFlagMaskCommand;
        default: return 0;
    }
}

static int mediaKey(uint16_t vk) {
    switch (vk) {
        case 0xad: return NX_KEYTYPE_MUTE;
        case 0xae: return NX_KEYTYPE_SOUND_DOWN;
        case 0xaf: return NX_KEYTYPE_SOUND_UP;
        case 0xb0: return NX_KEYTYPE_NEXT;
        case 0xb1: return NX_KEYTYPE_PREVIOUS;
        case 0xb3: return NX_KEYTYPE_PLAY;
        default: return -1;
    }
}

static bool postKey(uint16_t vk, bool down, CGEventFlags flags) {
    int media = mediaKey(vk);
    if (media >= 0) {
        int state = down ? NX_KEYDOWN : NX_KEYUP;
        NSEvent *event = [NSEvent otherEventWithType:NSEventTypeSystemDefined
                        location:NSZeroPoint modifierFlags:(flags | (state << 8)) timestamp:0
                        windowNumber:0 context:nil subtype:NX_SUBTYPE_AUX_CONTROL_BUTTONS
                        data1:(media << 16) | (state << 8) data2:-1];
        if (!event.CGEvent) return false;
        CGEventPost(kCGHIDEventTap, event.CGEvent);
        return true;
    }
    CGEventRef event = CGEventCreateKeyboardEvent(NULL, (CGKeyCode)keyCode(vk), down);
    if (!event) return false;
    CGEventSetFlags(event, flags);
    CGEventPost(kCGHIDEventTap, event);
    CFRelease(event);
    return true;
}

// 0 = success, 1 = missing Accessibility permission, 2 = unsupported key/event.
int keylume_mac_keys(const uint16_t *keys, size_t count) {
    @autoreleasepool {
        if (!CGPreflightPostEventAccess()) return 1;
        CGEventFlags flags = CGEventSourceFlagsState(kCGEventSourceStateCombinedSessionState);
        for (size_t i = 0; i < count; ++i) {
            flags |= modifier(keys[i]);
            if (!modifier(keys[i]) && keyCode(keys[i]) < 0 && mediaKey(keys[i]) < 0) return 2;
        }
        if (count > 5) return 2;
        bool generated[5] = {false};
        size_t pressed = 0;
        bool success = true;
        for (; pressed < count; ++pressed) {
            uint16_t vk = keys[pressed];
            if (modifier(vk)) continue;
            int key = keyCode(vk);
            if (key >= 0 && CGEventSourceKeyState(kCGEventSourceStateHIDSystemState, (CGKeyCode)key)) continue;
            if (!postKey(vk, true, flags)) {
                success = false;
                break;
            }
            generated[pressed] = true;
        }
        while (pressed > 0) {
            --pressed;
            if (generated[pressed] && !postKey(keys[pressed], false, flags)) success = false;
        }
        return success ? 0 : 2;
    }
}

int keylume_mac_scroll(int pixels) {
    if (!CGPreflightPostEventAccess()) return 1;
    CGEventRef event = CGEventCreateScrollWheelEvent(NULL, kCGScrollEventUnitPixel, 1, pixels);
    if (!event) return 2;
    CGEventPost(kCGHIDEventTap, event);
    CFRelease(event);
    return 0;
}

bool keylume_mac_volume(float *level, bool relative) {
    AudioObjectPropertyAddress address = {kAudioHardwarePropertyDefaultOutputDevice,
        kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain};
    AudioDeviceID device;
    UInt32 size = sizeof(device);
    if (AudioObjectGetPropertyData(kAudioObjectSystemObject, &address, 0, NULL, &size, &device)) return false;
    address.mSelector = kAudioHardwareServiceDeviceProperty_VirtualMainVolume;
    address.mScope = kAudioObjectPropertyScopeOutput;
    Float32 gain = 0;
    size = sizeof(gain);
    if (relative && AudioObjectGetPropertyData(device, &address, 0, NULL, &size, &gain)) return false;
    gain = fminf(1, fmaxf(0, gain + *level));
    if (AudioObjectSetPropertyData(device, &address, 0, NULL, size, &gain)) return false;
    *level = gain;
    return true;
}
