varying vec4 VaryingVertexPosition;

#ifdef VERTEX
extern float elapsed;
extern float width, height;

attribute float VertexIncrement;

vec4 position(mat4 transform_projection, vec4 vertex_position) {
  float phi = VertexIncrement * 6.28 + elapsed;

  float o = fract(sin(VertexIncrement) * 43758.5453);
  // float o = 0;

  vertex_position.x += cos(phi * 2 + elapsed) * width + cos(o) * 10;
  vertex_position.y += sin(phi * 3) * height + cos(o) * 10;
  vertex_position.z = 0.5 + cos(phi * 3) * 0.25;

  VaryingVertexPosition = vertex_position;

  return transform_projection * vertex_position;
}
#endif

#ifdef PIXEL
vec4 effect(vec4 color, Image texture, vec2 texture_coords, vec2 screen_coords) {
  vec2 tc = texture_coords - 0.5;
  tc *= 2.0;
  float c = 1.0 - length(tc);
  vec3 l = vec3(VaryingVertexPosition.z + 0.25);
  return vec4(c, c, c, c) * vec4(0.1, 0.5, 1.0, 1.0) * vec4(l, 1.0);
}
#endif
