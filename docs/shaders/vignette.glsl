#ifdef VERTEX
vec4 position(mat4 transform_projection, vec4 vertex_position) {
  return transform_projection * vertex_position;
}
#endif

#ifdef PIXEL
vec4 effect(vec4 color, Image texture, vec2 texture_coords, vec2 screen_coords) {
  vec2 uv = texture_coords * 2.0 - 1.0;
  float f = dot(uv, uv) * 0.7;
  return vec4(0.0, 0.0, 0.0, f);
}
#endif
