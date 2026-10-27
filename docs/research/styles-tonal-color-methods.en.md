# Tonal and color methods behind Photographic Styles

English | [简体中文](styles-tonal-color-methods.md)

The useful boundary is the pixel computation. Apple's Tone and Color controls locate native computations; they are not required public controls for an HDR implementation.

## Native mechanism

In the identified firmware consumer, Tone generates 17 background, 16 foreground and seven semantic coefficients. Branches depend on tuning and frame statistics. Positive and negative Tone are different generators, not inverses. The ordered consumer has distinct shadows, contrast, whites, mids/lighten, local-map and semantic protection leaves. Tone is not a single exposure multiplier.

Two positive-branch skin leaves are separately driven: highlight/color uses `2.75 t - 3.75 t²`; guide/brightness uses `a t² - b t`, with float32 `a=0x3e554c98`, `b=0x3f355326`. Here t is an internal scene-scaled input, not EV. A background local-map mix uses `max(1 - 0.6 t, 0.7)`. These coefficients can move in different directions. The skin-brightness leaf uses square-root RGB and a guide, luminance-dependent weighting, brightness protection and the skin matte. The highlight leaf adds luminance/chromaticity gates and warm-channel weighting.

Color has a separate path. An LMS-like transform, signed exponent near 0.43 and opponent intensity/chroma coordinates precede polar chroma adjustment and the inverse transform. The constants are IPT-like; a research function named `to_lab` does not imply CIELAB. The outer negative/nonpositive branch uses smooth saturation with gain `1+0.8 c`. The positive branch uses RichSat with `1+c²` and further lightness, hue and chromaticity weighting. A person-dependent blend protects part of the pre-color result. The complete cast/protection/output chain remains separate.

The conditional firmware arithmetic was compared at fixed upstream research revision `4a2ee56a66a598bee0e87d35d1146f6520734c03` (iPhone17,3/24A446). Actual Mac captures use macOS build 26A434 on M1 Pro. Native compute reflection confirms a 672-byte parameter block, 17/16/7 field blocks and texture roles: 10 foreground/background, 11 skin, 12 sky. Color comparisons change only the float at byte12; Tone comparisons change multiple groups. Other captured input buffers, inline bytes and textures are held identical. Fifty-seven cross-context checks of 19 statistics-independent positive fields have maximum discrepancy about `5.43e-8`. This does not prove every iOS/Mac path equivalent.

Direct interventions on retained native pipeline states change skin and background-local pixels while top-level Tone/Color remain zero. The person-local intervention is inactive in the legacy human context because its actual native foreground binding is zero. A legacy portrait matte elsewhere in the source is not proof that the native caller bound it as a semantic-person matte. Earlier region labels that used texture12 as person are invalid; texture12 is sky.

## Independent HDR implementation

`xdremux-hdr` provides independently callable midtone, skin and opponent-chroma composition. The optional [display outcome](../styles-reconstruction.en.md#separate-hdr-display-target) uses explicit method coefficients, source SDR chroma, declared headroom and role-qualified same-photo masks. It does not use a lifted native output difference field. The native reverse decoder still supplies the photo-native scene reconstruction inputs. The display path authors a coupled family with a scene/SDR range-gap proxy and a separately scheduled coarse spatial residual. It does not add the independent midtone or skin-brightness leaves to that family's range budget. The display guide defines the equation, single-map compromise, and seven-headroom native checks.

The method domain is chosen explicitly as component-linear Display P3, normalized by display headroom after rolloff. A captured Mac caller uses transfer code0/identity CCM on its relevant image slots; that observation binds the tested caller, not every upstream working space. Literal opponent coefficients are not an ICC conversion or universal P3/IPT equivalence claim. The implemented chroma composition is a smooth chroma/source-reference method; it is not a complete RichSat or complete Apple renderer port.

Frozen synthetic vectors compare the portable implementation with independent float arithmetic and a Metal port, including HDR, black and extended source SDR. Semantic extraction tests distinguish exact linked roles, explicit portrait proxies and ambiguity. Real-photo/native readback remains necessary for the execution path. Float-target agreement, encoded transport, unchanged SDR, RAW-reference scene accuracy and physical HDR appearance are separate evidence classes.

The implementation retains source texture/detail without rerunning Apple's texture renderer. Its own guide and SDR chroma anchoring are declared rendering choices. It does not claim complete camera ISP inversion, RAW recovery, calibrated radiance, native-guided-filter equivalence or physical-display visual acceptance.
