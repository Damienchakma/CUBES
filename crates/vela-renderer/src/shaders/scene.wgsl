struct Globals {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_direction: vec4<f32>,
    light_color: vec4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) model_0: vec4<f32>,
    @location(4) model_1: vec4<f32>,
    @location(5) model_2: vec4<f32>,
    @location(6) model_3: vec4<f32>,
    @location(7) base_color: vec4<f32>,
    @location(8) material: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) base_color: vec4<f32>,
    @location(4) material: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var base_color_texture: texture_2d<f32>;
@group(1) @binding(1) var base_color_sampler: sampler;

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    let model = mat4x4<f32>(
        input.model_0,
        input.model_1,
        input.model_2,
        input.model_3,
    );
    let world_position = model * vec4<f32>(input.position, 1.0);
    var output: VertexOutput;
    output.clip_position = globals.view_projection * world_position;
    output.world_position = world_position.xyz;
    output.normal = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);
    output.uv = input.uv;
    output.base_color = input.base_color;
    output.material = input.material;
    return output;
}

fn distribution_ggx(normal: vec3<f32>, halfway: vec3<f32>, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha_squared = alpha * alpha;
    let normal_dot_halfway = max(dot(normal, halfway), 0.0);
    let denominator = normal_dot_halfway * normal_dot_halfway * (alpha_squared - 1.0) + 1.0;
    return alpha_squared / max(3.14159265 * denominator * denominator, 0.0001);
}

fn geometry_schlick(normal_dot_view: f32, roughness: f32) -> f32 {
    let k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    return normal_dot_view / max(normal_dot_view * (1.0 - k) + k, 0.0001);
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let view_direction = normalize(globals.camera_position.xyz - input.world_position);
    let light_direction = normalize(-globals.light_direction.xyz);
    let halfway = normalize(view_direction + light_direction);
    let normal_dot_light = max(dot(normal, light_direction), 0.0);
    let normal_dot_view = max(dot(normal, view_direction), 0.0);

    let metallic = clamp(input.material.x, 0.0, 1.0);
    let roughness = clamp(input.material.y, 0.04, 1.0);
    let texel = textureSample(base_color_texture, base_color_sampler, input.uv);
    let base_color = input.base_color.rgb * texel.rgb;
    let f0 = mix(vec3<f32>(0.04), base_color, metallic);
    let distribution = distribution_ggx(normal, halfway, roughness);
    let geometry = geometry_schlick(normal_dot_view, roughness)
        * geometry_schlick(normal_dot_light, roughness);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - max(dot(halfway, view_direction), 0.0), 5.0);
    let specular = distribution * geometry * fresnel
        / max(4.0 * normal_dot_view * normal_dot_light, 0.001);
    let diffuse = (vec3<f32>(1.0) - fresnel)
        * (1.0 - metallic)
        * base_color
        / 3.14159265;
    let direct = (diffuse + specular) * globals.light_color.rgb * normal_dot_light;

    let ambient = base_color * 0.16;
    let emissive = base_color * max(input.material.z, 0.0);
    let mapped = ambient + direct + emissive;
    let gamma = pow(max(mapped, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
    return vec4<f32>(gamma, input.base_color.a * texel.a);
}
