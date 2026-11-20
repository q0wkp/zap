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
