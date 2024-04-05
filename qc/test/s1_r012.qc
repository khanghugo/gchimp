// Created by Crowbar 0.68

$modelname "cyberwave/ramps/s1_r012.mdl"

$staticprop

$bodygroup "Body"
{
	studio "s1_r012_ref.smd"
}


$surfaceprop "no_decal"

$contents "solid"

$illumposition -67.625 104.76 -0.005

$cdmaterials "models\cyberwave\"

$texturegroup "skinfamilies"
{
	{ "metal_light_01_dark" "mefl2_02_dark" "chromatic_glass_01" "neon_yellow_pulse" "circuit_board_yellow" "mefl1_02_dark" }
}

$cbox 0 0 0 0 0 0

$bbox -704.25 -1168.952 -272.259 495.592 1024 272.25

$definebone "static_prop" "" 0 0 0 0 0 0 0 0 0 0 0 0


$sequence "idle" {
	"s1_r012_anims\idle.smd"
	fadein 0.2
	fadeout 0.2
	fps 30
}

$collisionmodel "s1_r012_physics.smd"
{
	$mass 932862.8
	$inertia 1
	$damping 0
	$rotdamping 0
	$rootbone " "
	$concave
	$maxconvexpieces 20

}
