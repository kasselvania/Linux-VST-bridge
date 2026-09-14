#pragma once
#include "record.h"
#include <windows.h>
namespace uio1 {
// Production bounded scalar encoder. Readers never transfer handle ownership.
// The fixture substitutes these API reads, not fabricated User32 messages.
template<class PointerRead,class TouchRead>
void input_details(Header& h,Record* slots,const Record& r,UINT msg,WPARAM wp,LPARAM lp,
                   PointerRead pointer_read,TouchRead touch_read) noexcept {
    if((msg==WM_POINTERDOWN||msg==WM_POINTERUPDATE||msg==WM_POINTERUP||msg==WM_POINTERCAPTURECHANGED)){
      auto detail=r;detail.source=11;detail.x=GET_POINTERID_WPARAM(wp);
      detail.y=0;detail.buttons=HIWORD(wp);detail.key_class=0;detail.result=0;
      POINTER_INFO info{};
      if(pointer_read(UINT(detail.x),&info)){
        detail.y=int32_t(info.pointerType);detail.buttons=info.pointerFlags;detail.key_class=1;
        detail.screen_x=info.ptPixelLocation.x;detail.screen_y=info.ptPixelLocation.y;
      }else detail.result=GetLastError();
      uio1::append(h,slots,detail);
    }
    if(msg==WM_TOUCH){
      const auto count=LOWORD(wp);TOUCHINPUT contacts[16]{};
      if(count>0&&count<=16&&touch_read(reinterpret_cast<HTOUCHINPUT>(lp),count,contacts,sizeof(TOUCHINPUT))){
        for(unsigned i=0;i<count;++i){auto detail=r;detail.source=12;
          detail.x=int32_t(contacts[i].dwID);detail.y=int32_t(count);detail.buttons=contacts[i].dwFlags;
          detail.screen_x=contacts[i].x;detail.screen_y=contacts[i].y;detail.key_class=1;detail.result=0;
          uio1::append(h,slots,detail);
        }
      }else{auto detail=r;detail.source=12;detail.x=0;detail.y=count;detail.key_class=0;
        detail.result=count>16?ERROR_INSUFFICIENT_BUFFER:(count==0?ERROR_INVALID_PARAMETER:GetLastError());
        uio1::append(h,slots,detail);
      }
    }
}
}
