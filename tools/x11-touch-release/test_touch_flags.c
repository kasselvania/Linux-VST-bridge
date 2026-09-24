/* Compile against the header installed by wine-x11-touch-release.patch. */
#include <assert.h>

#define XI_RawTouchBegin 1
#define XI_RawTouchUpdate 2
#define XI_RawTouchEnd 3

#define POINTER_MESSAGE_FLAG_NEW 0x0001
#define POINTER_MESSAGE_FLAG_INRANGE 0x0002
#define POINTER_MESSAGE_FLAG_INCONTACT 0x0004

#include "touch_message_flags.h"

int main(void)
{
    assert(x11drv_touch_message_flags(XI_RawTouchBegin, POINTER_MESSAGE_FLAG_NEW)
           == (POINTER_MESSAGE_FLAG_INRANGE | POINTER_MESSAGE_FLAG_INCONTACT | POINTER_MESSAGE_FLAG_NEW));
    assert(x11drv_touch_message_flags(XI_RawTouchUpdate, 0)
           == (POINTER_MESSAGE_FLAG_INRANGE | POINTER_MESSAGE_FLAG_INCONTACT));
    assert(x11drv_touch_message_flags(XI_RawTouchEnd, 0)
           == POINTER_MESSAGE_FLAG_INRANGE);
    return 0;
}
