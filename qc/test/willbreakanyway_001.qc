// Created by Crowbar 0.68

$modelname "props/willbreakanyway_001.mdl"

$staticprop

$bodygroup "Body"
{
	studio "willbreakanyway_001_ref.smd"
}


$surfaceprop "no_decal"

$contents "solid"

$illumposition -160.184 -75.804 -0.014

$cdmaterials "models\props\willbreakanyway_001\"

$texturegroup "skinfamilies"
{
	{ "tilefloor01" "marblefloor001b" "metalflat" "gridwall_glow" "unbreakable" }
}

$cbox 0 0 0 0 0 0

$bbox -1487.856 -1520.302 -1093.519 1639.464 1199.933 1093.491

$definebone "static_prop" "" 0 0 0 0 0 0 0 0 0 0 0 0


$sequence "idle" {
	"willbreakanyway_001_anims\idle.smd"
	fadein 0.2
	fadeout 0.2
	fps 30
}
