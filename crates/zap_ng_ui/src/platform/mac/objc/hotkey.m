// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>

#import "hotkey.h"

@implementation ZapNGHotKey

- (instancetype)initWithEventHotKey:(EventHotKeyRef)eventHotKey
                            keyCode:(NSUInteger)keyCode
                       modifierKeys:(NSUInteger)modifierKeys {
    self = [super init];
    if (self) {
        _eventHotKey = eventHotKey;
        _keyCode = keyCode;
        _modifierKeys = modifierKeys;
    }
    return self;
}

- (BOOL)hotKeyKeyAndModifierEquals:(NSUInteger)keyCode modifierKeys:(NSUInteger)modifierKeys {
    return keyCode == _keyCode && modifierKeys == _modifierKeys;
}

@end
