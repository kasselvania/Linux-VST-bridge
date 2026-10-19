/* Compile against the production header in the exact patched Wine source. */
#include <assert.h>
#include <X11/extensions/XInput2.h>

#define POINTER_MESSAGE_FLAG_NEW 0x0001
#define POINTER_MESSAGE_FLAG_INRANGE 0x0002
#define POINTER_MESSAGE_FLAG_INCONTACT 0x0004
#include "touch_message_flags.h"

int main(void)
{
    assert(x11drv_touch_message_flags(XI_TouchBegin, POINTER_MESSAGE_FLAG_NEW)
           == (POINTER_MESSAGE_FLAG_INRANGE | POINTER_MESSAGE_FLAG_INCONTACT | POINTER_MESSAGE_FLAG_NEW));
    assert(x11drv_touch_message_flags(XI_TouchUpdate, 0)
           == (POINTER_MESSAGE_FLAG_INRANGE | POINTER_MESSAGE_FLAG_INCONTACT));
    assert(x11drv_touch_message_flags(XI_TouchEnd, 0) == POINTER_MESSAGE_FLAG_INRANGE);
    assert(x11drv_touch_message_flags(XI_RawTouchEnd, 0) == POINTER_MESSAGE_FLAG_INRANGE);
    return 0;
}
