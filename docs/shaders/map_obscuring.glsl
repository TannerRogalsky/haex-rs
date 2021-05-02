#ifdef VERTEX
vec4 position(mat4 transform_projection, vec4 vertex_position) {
  return transform_projection * vertex_position;
}
#endif

#ifdef PIXEL
extern float elapsed;
extern vec2 grid_dimensions;

float hash(vec2 p) {
    p = 50.0*fract( p*0.3183099 + vec2(0.71,0.113));
    return -1.0+2.0*fract( p.x*p.y*(p.x+p.y) );
}

float noise( in vec2 p ) {
  vec2 i = floor( p );
  vec2 f = fract( p );

  vec2 u = f*f*(3.0-2.0*f);
  return mix( mix( hash( i + vec2(0.0,0.0) ),
                   hash( i + vec2(1.0,0.0) ), u.x),
              mix( hash( i + vec2(0.0,1.0) ),
                   hash( i + vec2(1.0,1.0) ), u.x), u.y);
}

vec4 effect(vec4 color, Image texture, vec2 tc, vec2 screen_coords) {
  float line_width = 0.05;

  // bottom-left
  vec2 bl = step(vec2(line_width), fract(tc));
  float pct = bl.x * bl.y;

  // top-right
  vec2 tr = step(vec2(line_width), 1.0 - fract(tc));
  pct *= tr.x * tr.y;

  vec2 uv = tc * love_ScreenSize.xy + elapsed / 10.0;
  mat2 m = mat2( 1.6,  1.2, -1.2,  1.6 );
  float f = 0.0;
  f += 0.5000*noise(uv); uv = m*uv;
  f += 0.2500*noise(uv); uv = m*uv;
  f += 0.1250*noise(uv); uv = m*uv;
  f += 0.0625*noise(uv); uv = m*uv;

  float c = mix(
    1.0 - pct * (1.0 - min(1.0, f * 2.5) * Texel(texture, tc / grid_dimensions).r),
    1.0 - pct * (1.0 - f),
    pow(sin(elapsed / 10.0), 2.0)
  );
  return color * vec4(vec3(c), 1.0);
}
#endif
