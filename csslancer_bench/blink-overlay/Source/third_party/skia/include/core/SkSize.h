/*
 * Copyright 2011 Google Inc.
 *
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 */
 #ifndef SkSize_DEFINED
 #define SkSize_DEFINED

 ///////////////////////////////////////////////////////////////////////////////
 struct SkSize {
     float fWidth;
     float fHeight;
     static constexpr SkSize Make(float w, float h) { return {w, h}; }
     static constexpr SkSize MakeEmpty() { return {0, 0}; }
     void set(float w, float h) { *this = SkSize{w, h}; }
     /** Returns true iff fWidth == 0 && fHeight == 0
      */
     bool isZero() const { return 0 == fWidth && 0 == fHeight; }
     /** Returns true if either width or height are <= 0 */
     bool isEmpty() const { return fWidth <= 0 || fHeight <= 0; }
     /** Set the width and height to 0 */
     void setEmpty() { *this = SkSize{0, 0}; }
     float width() const { return fWidth; }
     float height() const { return fHeight; }
     bool equals(float w, float h) const { return fWidth == w && fHeight == h; }

 };
 static inline bool operator==(const SkSize& a, const SkSize& b) {
     return a.fWidth == b.fWidth && a.fHeight == b.fHeight;
 }
 static inline bool operator!=(const SkSize& a, const SkSize& b) { return !(a == b); }
 #endif