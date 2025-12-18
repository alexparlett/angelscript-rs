# Failure: conversion-lookup

Used class.find_methods() and iterated by method name to find conversion operators when should use behaviors.operators with OperatorBehavior as key for O(1) lookup. Conversion operators (OpConv, OpImplConv, OpCast, OpImplCast) include target TypeHash in the enum variant which enables direct keyed lookup.
