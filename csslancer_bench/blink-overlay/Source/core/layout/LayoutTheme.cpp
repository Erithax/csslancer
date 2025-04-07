/**
 * This file is part of the theme implementation for form controls in WebCore.
 *
 * Copyright (C) 2005, 2006, 2007, 2008, 2009, 2010, 2012 Apple Computer, Inc.
 *
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Library General Public
 * License as published by the Free Software Foundation; either
 * version 2 of the License, or (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Library General Public License for more details.
 *
 * You should have received a copy of the GNU Library General Public License
 * along with this library; see the file COPYING.LIB.  If not, write to
 * the Free Software Foundation, Inc., 51 Franklin Street, Fifth Floor,
 * Boston, MA 02110-1301, USA.
 */

//  #include "config.h"
//  #include "core/layout/LayoutTheme.h"
 
//  #include "core/CSSValueKeywords.h"
//  #include "core/HTMLNames.h"
//  #include "core/InputTypeNames.h"
//  #include "core/dom/Document.h"
//  #include "core/dom/shadow/ElementShadow.h"
//  #include "core/editing/FrameSelection.h"
//  #include "core/fileapi/FileList.h"
//  #include "core/frame/LocalFrame.h"
//  #include "core/frame/Settings.h"
//  #include "core/html/HTMLCollection.h"
//  #include "core/html/HTMLDataListElement.h"
//  #include "core/html/HTMLDataListOptionsCollection.h"
//  #include "core/html/HTMLFormControlElement.h"
//  #include "core/html/HTMLInputElement.h"
//  #include "core/html/HTMLMeterElement.h"
//  #include "core/html/HTMLOptionElement.h"
//  #include "core/html/parser/HTMLParserIdioms.h"
//  #include "core/html/shadow/MediaControlElements.h"
//  #include "core/html/shadow/ShadowElementNames.h"
//  #include "core/html/shadow/SpinButtonElement.h"
//  #include "core/html/shadow/TextControlInnerElements.h"
//  #include "core/page/FocusController.h"
//  #include "core/page/Page.h"
//  #include "core/style/ComputedStyle.h"
//  #include "platform/FileMetadata.h"
//  #include "platform/FloatConversion.h"
//  #include "platform/RuntimeEnabledFeatures.h"
//  #include "platform/fonts/FontSelector.h"
//  #include "platform/text/PlatformLocale.h"
//  #include "platform/text/StringTruncator.h"
//  #include "public/platform/Platform.h"
//  #include "public/platform/WebFallbackThemeEngine.h"
//  #include "public/platform/WebRect.h"
//  #include "wtf/text/StringBuilder.h"
 
#if USE(NEW_THEME)
#include "platform/Theme.h"
#endif

// The methods in this file are shared by all themes on every platform.

namespace blink {

using namespace HTMLNames;

LayoutTheme::LayoutTheme()
    : m_hasCustomFocusRingColor(false)
#if USE(NEW_THEME)
    , m_platformTheme(platformTheme())
#endif
{
}


static FontDescription& getCachedFontDescription(CSSValueID systemFontID)
{
    DEFINE_STATIC_LOCAL(FontDescription, caption, ());
    DEFINE_STATIC_LOCAL(FontDescription, icon, ());
    DEFINE_STATIC_LOCAL(FontDescription, menu, ());
    DEFINE_STATIC_LOCAL(FontDescription, messageBox, ());
    DEFINE_STATIC_LOCAL(FontDescription, smallCaption, ());
    DEFINE_STATIC_LOCAL(FontDescription, statusBar, ());
    DEFINE_STATIC_LOCAL(FontDescription, webkitMiniControl, ());
    DEFINE_STATIC_LOCAL(FontDescription, webkitSmallControl, ());
    DEFINE_STATIC_LOCAL(FontDescription, webkitControl, ());
    DEFINE_STATIC_LOCAL(FontDescription, defaultDescription, ());
    switch (systemFontID) {
    case CSSValueCaption:
        return caption;
    case CSSValueIcon:
        return icon;
    case CSSValueMenu:
        return menu;
    case CSSValueMessageBox:
        return messageBox;
    case CSSValueSmallCaption:
        return smallCaption;
    case CSSValueStatusBar:
        return statusBar;
    case CSSValueWebkitMiniControl:
        return webkitMiniControl;
    case CSSValueWebkitSmallControl:
        return webkitSmallControl;
    case CSSValueWebkitControl:
        return webkitControl;
    case CSSValueNone:
        return defaultDescription;
    default:
        ASSERT_NOT_REACHED();
        return defaultDescription;
    }
}

void LayoutTheme::systemFont(CSSValueID systemFontID, FontDescription& fontDescription)
{
    fontDescription = getCachedFontDescription(systemFontID);
    if (fontDescription.isAbsoluteSize())
        return;

    FontStyle fontStyle = FontStyleNormal;
    FontWeight fontWeight = FontWeightNormal;
    float fontSize = 0;
    AtomicString fontFamily;
    systemFont(systemFontID, fontStyle, fontWeight, fontSize, fontFamily);
    fontDescription.setStyle(fontStyle);
    fontDescription.setWeight(fontWeight);
    fontDescription.setSpecifiedSize(fontSize);
    fontDescription.setIsAbsoluteSize(true);
    fontDescription.firstFamily().setFamily(fontFamily);
    fontDescription.setGenericFamily(FontDescription::NoFamily);
}




} // namespace blink
