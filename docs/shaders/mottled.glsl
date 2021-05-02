#ifdef VERTEX
extern float z;

vec4 position(mat4 transform_projection, vec4 vertex_position) {
  vertex_position.z = z;
  return transform_projection * vertex_position;
}
#endif

#ifdef PIXEL
extern float scale;
extern float strength;

float hash(vec2 p)  // replace this by something better
{
    p  = 50.0*fract( p*0.3183099 + vec2(0.71,0.113));
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

vec4 effect(vec4 color, Image texture, vec2 texture_coords, vec2 screen_coords) {
  vec2 uv = texture_coords * scale;
  mat2 m = mat2( 1.6,  1.2, -1.2,  1.6 );
  float f = 0.0;
  f  = 0.5000*noise( uv ); uv = m*uv;
  f += 0.2500*noise( uv ); uv = m*uv;
  f += 0.1250*noise( uv ); uv = m*uv;
  f += 0.0625*noise( uv ); uv = m*uv;

  float l = texture_coords.x;
  float c = 1.0 - (pow(l, 5.0) + pow(1.0 - l, 5.0));

  vec4 texturecolor = Texel(texture, texture_coords) - vec4(vec3(f / strength), 0.0);
  return texturecolor * color * c;
}
#endif
