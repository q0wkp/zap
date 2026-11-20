// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>

@interface NSPasteboard (ZapNG)
- (NSArray *)getFilePaths;
@end

/// ZapNGHostView is the Content view of a ZapNG window.
// It is backed by a Metal CALayer.
@interface ZapNGHostView : NSView <CALayerDelegate, NSTextInputClient>
- (ZapNGHostView *)initWithFrame:(NSRect)frame
                    metalDevice:(id)metalDevice
             enableTitlebarDrag:(BOOL)enableTitlebarDrag
                       testMode:(BOOL)testMode;
- (void)setAsyncCallback:(BOOL)shouldAsync;
- (BOOL)keyDownImpl:(NSEvent *)event;
@end
