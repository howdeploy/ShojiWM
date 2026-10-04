uniform float pixelSize;

vec4 shader_main(EffectContext effect) {
    vec2 size = max(effect.texture_size_px, vec2(1.0));
    float block = max(pixelSize, 1.0);
    vec2 pixel = (floor(effect.texture_uv * size / block) + 0.5) * block;
    vec2 uv = clamp(pixel, vec2(0.5), size - 0.5) / size;
    return texture2D(tex, uv);
}
