#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>
#import <stdint.h>

// The notification centers retain their block observers for the application's lifetime.
void keylume_observe_session(void (*sleep_changed)(bool), void (*lock_changed)(bool)) {
    NSNotificationCenter *workspace = NSWorkspace.sharedWorkspace.notificationCenter;
    NSDistributedNotificationCenter *distributed = NSDistributedNotificationCenter.defaultCenter;
    [workspace addObserverForName:NSWorkspaceWillSleepNotification object:nil queue:nil
                       usingBlock:^(__unused NSNotification *note) { sleep_changed(true); }];
    [workspace addObserverForName:NSWorkspaceDidWakeNotification object:nil queue:nil
                       usingBlock:^(__unused NSNotification *note) { sleep_changed(false); }];
    [distributed addObserverForName:@"com.apple.screenIsLocked" object:nil queue:nil
                         usingBlock:^(__unused NSNotification *note) { lock_changed(true); }];
    [distributed addObserverForName:@"com.apple.screenIsUnlocked" object:nil queue:nil
                         usingBlock:^(__unused NSNotification *note) { lock_changed(false); }];
}

uint32_t keylume_foreground_pid(void) {
    @autoreleasepool {
        return (uint32_t)NSWorkspace.sharedWorkspace.frontmostApplication.processIdentifier;
    }
}

uint32_t keylume_idle_minutes(void) {
    double seconds = CGEventSourceSecondsSinceLastEventType(
        kCGEventSourceStateCombinedSessionState, kCGAnyInputEventType);
    return seconds > 0 && seconds < UINT32_MAX ? (uint32_t)(seconds / 60) : 0;
}
